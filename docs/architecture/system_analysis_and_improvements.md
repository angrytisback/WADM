# WADM - Sistem Analizi ve İyileştirme Raporu (Technical Debt & Risks)

## 1. Güvenlik Açıkları ve Riskler (Kritik)

### A. Hardcoded JWT Secret
* **Dosya/Satır:** `src/api/auth.rs`, `src/middleware.rs` ve `src/api/terminal.rs`
* **Sorun:** Kod tabanında JWT anahtarı olarak sabit string kullanılmış: `let secret = b"super_secret_key_change_this_in_prod";`. Bu, uygulamaya erişen veya source code'u okuyan herkesin kendi admin token'ını (JWT) imzalayıp sisteme sızmasına (Authentication Bypass) neden olur.
* **Çözüm:** JWT secret random byte array olarak çalışma zamanında (runtime) veya kurulum aşamasında üretilip, `wadm-auth.json` veya environment file (.env) içerisine kalıcı olarak saklanmalıdır.

### B. Route Tanımlama Hatası ve Authentication Bypass
* **Dosya:** `src/main.rs` (Satır 62-86 arası)
* **Sorun:** `/api/packages/*` rotaları (list, upgrade, remove vb.) `App::new().route()` çağrısıyla doğrudan root kapsamına eklenmiş. Ancak Actix'te `Auth` middleware'i yalnızca `web::scope("/api").wrap(Auth)` modül bloğuna tanımlanmış. Bu mimari hata, yetkisiz herhangi bir kullanıcının API endpointlerine istek atarak sistemdeki paketleri yükleyip silebilmesine olanak sağlar!
* **Çözüm:** Tüm private rotalar istisnasız `web::scope("/api").wrap(Auth)` içerisindeki `.configure(api::config)` adımında tanıtılmalıdır. `main.rs` tarafında yapılan manuel eklemeler `api/mod.rs` içerisine taşınmalıdır.

### C. Command ve Shell Injection (Remote Code Execution - RCE)
* **Dosya:** `src/api/pkgmgr.rs`, `src/api/services.rs` ve `src/api/db.rs`
* **Sorun:** Proje genelinde OS komutları `Command::new("sudo").args(...)` ile çalıştırılıyor.
  * **Argument Injection:** `pkgmgr.rs` içindeki `upgrade_package_impl` ve `services.rs` içindeki `control_service` fonsiyonlarında, kullanıcıdan gelen `name` ve `action` parametreleri doğrudan argüman dizisine geçiriliyor. (`apt-get install <name>`). Zararlı bir kullanıcı paket ismi olarak `-oAPT::Update::Pre-Invoke::=/bin/sh -c 'pwned'` gönderirse doğrudan kod çalıştırabilir.
  * **Shell Injection:** En tehlikelisi, `db.rs` dosyasında backup/restore operasyonlarında: `Command::new("sh").args(&["-c", &format!("sudo -n -u postgres pg_dump {} > {}", db, filepath)])` kullanılıyor. Buradaki `db` değişkeni bir API JSON'ından geldiği için `test_db; rm -rf /;` şeklinde gönderilen bir string sistemi çökertecek bir komut enjeksiyonuna (Command Injection) neden olur.
* **Çözüm:** Girdi sanitizasyonu (validation/Regex) zorunludur. DB, Container veya Paket isimleri Regex (`^[a-zA-Z0-9_-]+$`) ile doğrulanmalı, argüman birleştirmek (string formatting) yerine native flag binding yapılmalıdır.

## 2. Performans Darboğazları

### A. Senkron Tokip/Actix Bloking (Thread Starvation)
* **Sorun:** Projede neredeyse her API çağrısında `std::process::Command` senkron olarak kullanılmış. Actix Web asenkron tokio worker threadleri üzerinde çalışır. Senkron process başlatmak ve `.output()` metodunu çağırmak, bağlı bulunduğu tokio worker thread'ini işletim sistemi işlemi bitene kadar kilitler (block). 
* **Etki:** Örneğin `api::monitor.rs` içerisindeki `get_system_stats` rotası, içindeki `count_upgradable_packages()` methodu ile `apt-get list` çağrısını senkron yapar. Bir güncelleme denetimi bazen saniyeler sürer. Eğer kullanıcı arayüzü her birkaç saniyede bir polling yaparsa, actix worker'ları kilitleneceği için tüm sunucu diğer API isteklerini reddeder hale gelir.
* **Çözüm:** Tüm OS komutları tokio'nun `tokio::process::Command` kütüphanesiyle asenkrona (`await`) çevrilmelidir. Veya senkron kalacaksa mutlaka `actix_web::web::block(move || { ... })` sarmalayıcısına alınarak ayrı bir thread pool (rayon/tokio blocking pool) üzerinden işletilmelidir.

## 3. Kod Kalitesi ve Refactoring Önerileri

* **SOLID İhlalleri (God Functions):** `monitor.rs` dosyasındaki `get_gpu_stats`, devasa boyutlara ulaşmış ve donanım bağımlı çok fazla hardcoded algoritma barındırıyor (Nvidia, AMD, Intel). Bu sistem `GpuMonitor` trait'i altına `NvidiaMonitor`, `AmdMonitor` olarak implement edilip modülerleştirilmelidir.
* **Frontend Context Cehennemi:** Frontend'de `AuthContext`, `ToastContext`, `SystemContext`, `StatsContext` gibi iç içe (nested) bir ton provider var (`App.tsx`). Bu React tree yapısını şişirir ve render süreçlerini etkiler. Global state yönetimi için `Zustand` veya `Redux Toolkit` gibi bağımsız ve hafif bir kütüphaneye geçilmesi kodun okunabilirliğini ve performansını artırır.
* **Mutex Kullanımı:** Uygulamadaki global stateler (Örn: `AuthStore` veya `System`) `std::sync::Mutex` ile kilitleniyor. Bu, async bağlamlarda threadlerin (ve tokio runtime'ının) paniğine veya ölü kilitlenmelere (deadlock) yol açabilir. Asenkron state paylaşımlarında `tokio::sync::Mutex` veya `tokio::sync::RwLock` kullanılmalıdır.

## 4. Mimari İyileştirmeler ve Yeni Özellikler (Feature Ideas)

* **Gerçek Zamanlı Streaming (SSE veya WS):** Arayüz şu anda donanım izlemek veya paket listesi çekmek için sürekli HTTP üzerinden polling (periyodik istek) yapıyor. Polling yerine Server-Sent Events (SSE) ile backend'in değer değiştikçe frontend'e push yapması sistemi (özellikle I/O ve ağ) inanılmaz rahatlatacaktır.
* **Gelişmiş Veritabanı (DB) Bağlantısı:** `db.rs` modülü `mysql` veya `psql` binary'lerini komut satırından çalıştırarak `-e "SELECT *"` şeklinde veri çekiyor. Veritabanı sorguları bu şekilde yapıldığında tablo büyüdükçe crash verir, maliyetlidir ve SQL enjeksiyonlarına davetiye çıkarır. Bunun yerine Rust içinde `sqlx` (Rust ORM/Driver) modülü eklenip doğrudan soket üzerinden veritabanlarına kalıcı (pool) bağlantılarla sorgu çekilmelidir.
* **Audit Logs / Görev Zamanlayıcı:** Yapılan her kritik işlem (paket silme, servis kapatma, firewall kuralı ekleme) IP adresleri ve zaman damgasıyla bir SQlite üzerinde saklanıp arayüzden Audit Log olarak gösterilebilir. Ek olarak belirli saatlerde tetiklenecek "Otomatik Yedekleme" gibi cron-job altyapısı sisteme entegre edilebilir.

## 5. Hata Yönetimi (Error Handling) ve Loglama

* **Result::unwrap() Tuzakları:** Sistem genelinde çok fazla `unwrap()` (özellikle TOTP doğrulama veya JWT ayrıştırma esnasında, `auth.rs`) mevcut. Hatalı bir `wadm-auth.json` dosyası veya yanlış base64 parse işlemi sonucu program sessizce "panic" olup çöküyor.
* **Error Enum Yapısı:** Rust tarafında özel bir `WadmError` struct'ı (veya `thiserror` kütüphanesi kullanılarak) oluşturulmalı ve `actix_web::ResponseError` trait'i implement edilmelidir. Şu anda hatalar `HttpResponse::InternalServerError().json(String)` olarak dağınık dönüyor. RFC 7807 (Problem Details for HTTP APIs) yapısına uygun standart bir JSON hata objesine dönüştürülmesi frontend için hata yakalamayı çok daha modülerleştirecektir.
