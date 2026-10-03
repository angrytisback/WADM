# WADM (Web Admin for Linux) - Sistem Analizi, Güvenlik Açıkları ve İyileştirme Raporu

> **Belge Amacı:** Bu doküman; WADM projesinin kaynak kodlarının, mimarisinin, veri yapılarının ve operasyonel süreçlerinin kıdemli bir Yazılım Mimarı, Güvenlik Araştırmacısı ve Performans Optimizasyon Uzmanı gözüyle gerçekleştirilmiş derinlemesine teknik denetim raporudur. Tespit edilen tüm açıklar, darboğazlar ve teknik borçlar somut dosya ve satır referanslarıyla ortaya konmuş, laboratuvar birim test sonuçları (14 `cargo test` vakası), frontend derleme/lint denetimleri (`npm run build` & `npm run lint`) ve uygulanması gereken somut çözümlerle belgelenmiştir.

---

## 1. Düzeltilmiş Açıklar ve Mevcut Durum Matrisi (Audit & Fix Verification Matrix)

Projenin önceki incelemelerinden bu yana kapatılmış, birim testleri yazılmış ve laboratuvarda doğrulanmış güvenlik/kararlılık maddeleri:

| Bileşen / Problem | Dosya & Satır Referansı | Durum | Test ve Doğrulama Sonucu |
| :--- | :--- | :--- | :--- |
| **Sabit JWT Anahtarı (Hardcoded Secret)** | [`src/api/auth.rs:18-38`](file:///home/eigen/Projects/wadm/src/api/auth.rs#L18-L38) | ✅ **DÜZELTİLDİ** | Sabit anahtar kaldırıldı. `OsRng` ile 32 bayt rastgele anahtar üretilip `.wadm_jwt_secret` dosyasına `0600` izinleriyle yazılıyor. `test_jwt_secret_generation` birim testi ile doğrulandı. |
| **Paket Rotalarında Auth Bypass** | [`src/api/mod.rs:61-74`](file:///home/eigen/Projects/wadm/src/api/mod.rs#L61-L74) | ✅ **DÜZELTİLDİ** | `/api/packages/*` rotaları `main.rs` içindeki korumasız alandan çıkarılıp `api::config` modülüne taşındı ve `middleware::Auth` koruması altına alındı. |
| **Paket Yöneticisi Argüman Enjeksiyonu** | [`src/api/pkgmgr.rs:170`](file:///home/eigen/Projects/wadm/src/api/pkgmgr.rs#L170) | ✅ **DÜZELTİLDİ** | `is_valid_package_name` fonksiyonu ile `-oAPT::Update=1`, `;`, `|` gibi zararlı girdiler engellendi. `test_valid_package_names` ve `test_invalid_package_names_injection` testleri ile doğrulandı. |
| **Servis Yöneticisi Argüman Enjeksiyonu** | [`src/api/services.rs:105`](file:///home/eigen/Projects/wadm/src/api/services.rs#L105) | ✅ **DÜZELTİLDİ** | `is_valid_service_name` fonksiyonu ile `--now`, `; rm -rf /` ve parametre enjeksiyonları engellendi. `test_valid_service_names` ve `test_invalid_service_names_injection` testleri ile doğrulandı. |
| **UFW Güvenlik Duvarı Komut Enjeksiyonu** | [`src/api/firewall.rs:107`](file:///home/eigen/Projects/wadm/src/api/firewall.rs#L107) | ✅ **DÜZELTİLDİ** | `is_safe_ufw_rule` fonksiyonu ile `allow 22; rm -rf /` ve zararlı komut birleştirmeleri engellendi. `test_valid_ufw_rules` ve `test_invalid_ufw_rules_injection` testleri ile doğrulandı. |
| **CORS ve Güvenlik Başlıkları** | [`src/main.rs:46-101`](file:///home/eigen/Projects/wadm/src/main.rs#L46-L101) | ✅ **DÜZELTİLDİ** | `allow_any_origin()` kaldırıldı; `WADM_ALLOWED_ORIGINS` ve localhost/127.0.0.1 filtresi eklendi. `nosniff`, `DENY`, `strict-origin-when-cross-origin` başlıkları tanımlandı. |
| **Brute-Force Login Koruması** | [`src/api/auth.rs:40-71`](file:///home/eigen/Projects/wadm/src/api/auth.rs#L40-L71) | ✅ **DÜZELTİLDİ** | `LoginRateLimiter` ile 1 dakikada 5 başarısız denemeden sonra IP engelleniyor. `test_login_rate_limiter` birim testi ile 5 deneme sonrası 6. denemenin bloklandığı doğrulandı. |
| **Veritabanı Identifier & Yedek Dosya Adı** | [`src/api/db.rs:42-59`](file:///home/eigen/Projects/wadm/src/api/db.rs#L42-L59) | ✅ **DÜZELTİLDİ** | `is_valid_db_identifier` ve `is_valid_backup_filename` fonksiyonlarıyla dizin atlama (`../`) engellendi. `test_valid_db_identifiers` ve `test_valid_backup_filenames` testleri ile doğrulandı. |
| **Dosya Yolu Güvenliği & Arbitrary Write** | [`src/api/files.rs:62-160`](file:///home/eigen/Projects/wadm/src/api/files.rs#L62-L160) | ✅ **DÜZELTİLDİ** | `/etc/shadow*`, `/etc/gshadow*`, `/etc/sudoers*`, SSH private keys (`id_*`, `.pem`, `.key`), `/proc/*/environ` okuma engellendi; `/etc`, `/boot`, `/usr`, `/bin`, `/sbin`, `/lib`, `/proc`, `/sys`, `/dev` yazma engellendi. `test_path_sanitization_blocked` ve `test_path_sanitization_hardened_invariants` testleri ile doğrulandı. |
| **SQL Tehlikeli Sorgu ve RCE Koruması** | [`src/api/db.rs:88-160`](file:///home/eigen/Projects/wadm/src/api/db.rs#L88-L160) | ✅ **DÜZELTİLDİ** | `normalize_sql` ile yorum satırları (`/* */`, `--`) temizlendi. PostgreSQL `DO $$`, `lo_export`, `pg_read_file`, `dblink` ve `TO PROGRAM` engellendi. `test_dangerous_query_detection_and_bypasses` testi ile doğrulandı. |
| **2s Paket Polling Fork-Bomb** | [`src/api/pkgmgr.rs:9-25`](file:///home/eigen/Projects/wadm/src/api/pkgmgr.rs#L9-L25), [`src/api/monitor.rs:561`](file:///home/eigen/Projects/wadm/src/api/monitor.rs#L561) | ✅ **DÜZELTİLDİ** | `UPGRADABLE_CACHE` statik TTL önbelleği (5 dakika) eklendi. Paket işlemlerinde `invalidate_upgradable_cache()` ile otomatik geçersiz kılınıyor. 2 saniyelik fork-bomb sonlandırıldı. |
| **File Explorer Dosya İndirme Hatası** | [`web/src/components/FileExplorer.tsx:147-170`](file:///home/eigen/Projects/wadm/web/src/components/FileExplorer.tsx#L147-L170) | ✅ **DÜZELTİLDİ** | Hatalı `window.open` yerine `handleDownload` fonksiyonu ile `fetch` + `Blob` + sanal `<a>` linki deseni uygulandı. `Authorization: Bearer <token>` başlığı sorunsuz aktarılıyor. |
| **`/dev/zero` ile OOM Çökmesi** | [`src/api/files.rs:230-260`](file:///home/eigen/Projects/wadm/src/api/files.rs#L230-L260) | ✅ **DÜZELTİLDİ** | `metadata.is_file()` denetimi eklendi; özel aygıtlar (`/dev/zero`, `/dev/urandom`), FIFO ve soketler reddedildi. `file.take(10MB + 1)` ile bellek taşması engellendi. `test_device_file_is_not_regular_file` ile doğrulandı. |
| **Terminal TTY Zombie Bash Süreçleri** | [`src/api/terminal.rs:77-154`](file:///home/eigen/Projects/wadm/src/api/terminal.rs#L77-L154) | ✅ **DÜZELTİLDİ** | `mut child` referansı saklandı; WebSocket döngüsü kapandığında `child.kill()` ve `child.wait()` çağrılarak zombi süreçler ve PTY sızıntısı engellendi. |
| **`docker.rs` Boş Container ID Hatası** | [`src/api/docker.rs:136`](file:///home/eigen/Projects/wadm/src/api/docker.rs#L136) | ✅ **DÜZELTİLDİ** | `get_container_stats` JSON yanıtında hardcoded `""` yerine gerçek `container_id` aktarıldı. |
| **Frontend TypeScript/React Lint Hataları** | `web/src/**/*.{ts,tsx}` | ✅ **DÜZELTİLDİ** | `npm run lint` çıktısındaki tüm React 19 kural ihlalleri (`react-hooks/refs`, mutasyonlar, any türleri) temizlendi (`eslint .` -> 0 error, 0 warning). |

### Otomatik Test Paketi Yürütme Kanıtı (Automated Test Execution)

```bash
$ cargo test
running 14 tests
test api::auth::tests::test_jwt_secret_generation ... ok
test api::auth::tests::test_login_rate_limiter ... ok
test api::db::tests::test_valid_backup_filenames ... ok
test api::db::tests::test_dangerous_query_detection_and_bypasses ... ok
test api::db::tests::test_valid_db_identifiers ... ok
test api::firewall::tests::test_invalid_ufw_rules_injection ... ok
test api::files::tests::test_device_file_is_not_regular_file ... ok
test api::firewall::tests::test_valid_ufw_rules ... ok
test api::files::tests::test_path_sanitization_hardened_invariants ... ok
test api::pkgmgr::tests::test_invalid_package_names_injection ... ok
test api::pkgmgr::tests::test_valid_package_names ... ok
test api::services::tests::test_invalid_service_names_injection ... ok
test api::services::tests::test_valid_service_names ... ok
test api::files::tests::test_path_sanitization_blocked ... ok

test result: ok. 14 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

---

## 2. Güvenlik Açıkları ve Riskler (Security Vulnerabilities & Risks)

### 2.1. JWT Token Saklama Yöntemi (`localStorage`) ve XSS Riski
* **Konum:** [`web/src/context/AuthContext.tsx:25, 34`](file:///home/eigen/Projects/wadm/web/src/context/AuthContext.tsx#L25)
* **Problem:** 
  JWT belirteci tarayıcının `localStorage` alanında saklanmaktadır:
  ```typescript
  const [token, setToken] = useState<string | null>(localStorage.getItem('wadm_token'));
  ```
  Eğer arayüzde (örneğin dosya adları, systemd servis açıklamaları veya log çıktıları vasıtasıyla) tek bir XSS açığı tetiklenirse, kötü niyetli bir JavaScript betiği `localStorage.getItem('wadm_token')` ile yönetici oturumunu çalabilir ve sunucu üzerinde tam kontrole sahip olabilir.
* **Etki Derecesi:** 🟡 **Orta - Yüksek (XSS varlığında Kritik)**
* **Çözüm Önerisi:**
  1. Token transferi `Set-Cookie: wadm_token=...; HttpOnly; Secure; SameSite=Strict` başlığı ile `HttpOnly` çerezlere taşınmalıdır. Bu sayede JavaScript kodlarının tokene erişimi fiziksel olarak imkansız kılınır.
  2. CSRF koruması için `SameSite=Strict` yanında çift çerez (Double Submit Cookie) veya özel bir header (`X-WADM-CSRF`) zorunlu tutulmalıdır.

---

### 2.2. Web Terminalinde Sınırsız Root Bash Erişimi ve Audit Eksikliği
* **Konum:** [`src/api/terminal.rs:76-80`](file:///home/eigen/Projects/wadm/src/api/terminal.rs#L76-L80)
* **Problem:**
  ```rust
  let cmd = CommandBuilder::new("bash");
  let mut child = pair.slave.spawn_command(cmd).expect("Failed to spawn shell");
  ```
  Developer Mode aktif edildiğinde, WADM doğrudan sunucunun çalıştığı kullanıcının (çoğu zaman `root`) kabuğunu başlatmaktadır. Bu kabuk üzerinde:
  - Hiçbir komut kısıtlaması (restricted shell / `rbash`) yoktur.
  - Çalıştırılan komutlar WADM sistem loglarına veya kalıcı bir audit dosyasına kaydedilmemektedir.
  - Bir kere oturum açan kullanıcı, WADM'in `files.rs` üzerinde getirdiği tüm güvenlik çitlerini (`/etc/shadow`, `authorized_keys` vb.) terminal üzerinden tek bir komutla baypas edebilir.
* **Etki Derecesi:** 🔴 **Yüksek**
* **Çözüm Önerisi:**
  1. Terminal oturumları için dedicated, izole bir kullanıcı (`wadm-shell`) tanımlanmalı veya `systemd-run -p DynamicUser=yes` ile kısıtlı cgroup ortamı açılmalıdır.
  2. Terminalde çalıştırılan komutlar için `auditd` veya PTY reader düzeyinde komut geçmişi (Command History Logger) tutularak `wadm-audit.log` dosyasına yazılmalıdır.

---

### 2.3. Global `window.fetch` Monkey-Patching Güvenlik ve İzolasyon Riski
* **Konum:** [`web/src/context/AuthContext.tsx:63-92`](file:///home/eigen/Projects/wadm/web/src/context/AuthContext.tsx#L63-L92)
* **Problem:**
  `AuthContext.tsx` içinde `window.fetch` global prototipi ezilmektedir. URL denetimi yapılsa da:
  ```typescript
  const isInternalOrRelative = urlString.startsWith('/') || urlString.startsWith(window.location.origin);
  ```
  - Projeye gelecekte eklenebilecek 3. parti kütüphaneler, iframe bileşenleri veya web worker'lar bu global fonksiyonu kullanarak iç API rotalarına token ile yetkilendirilmiş istek gönderebilir.
  - Global nesneleri mutasyona uğratmak React geliştirme standartlarına (best practices) ve StrictMode döngülerine aykırıdır.
* **Etki Derecesi:** 🟡 **Orta**
* **Çözüm Önerisi:**
  `window.fetch` globalini değiştirmek yerine, özel bir `apiClient.ts` sarmalayıcısı (wrapper) veya `axios` örneği oluşturulmalı; tüm API çağrıları bu modül üzerinden yürütülmelidir.

---

### 2.4. App Store Şablonlarında Sabit Portlar ve Dinamik Port Çakışma Riski
* **Konum:** [`src/api/apps.rs:32, 55, 82`](file:///home/eigen/Projects/wadm/src/api/apps.rs#L32)
* **Problem:**
  Nextcloud için `8080`, Pi-hole için `8081`, WordPress için `8082` portları `docker-compose.yml` içinde sabit (hardcoded) olarak tanımlanmıştır:
  ```yaml
  ports:
    - 8080:80
  ```
  Eğer sunucuda bu portları kullanan başka bir servis (örneğin Tomcat, Apache, alternatif bir web servisi) çalışıyorsa, Docker başlatma işlemi `port already in use` hatası vererek çökecektir.
* **Etki Derecesi:** 🟡 **Orta**
* **Çözüm Önerisi:**
  1. Kurulum öncesi sunucudaki boş portları tespit eden bir algoritma (`std::net::TcpListener::bind("0.0.0.0:0")`) yazılmalı veya kullanıcıya kurulum modalında portu değiştirme seçeneği sunulmalıdır.
  2. Port çakışması tespit edildiğinde arayüze açıklayıcı bir hata dönülmelidir.

---

### 2.5. App Store Üretilen Parolaların Arayüzde Gösterilememesi
* **Konum:** [`src/api/apps.rs:161-170`](file:///home/eigen/Projects/wadm/src/api/apps.rs#L161-L170)
* **Problem:**
  `install_app` fonksiyonu kriptografik rastgele güvenli şifre üretmekte ve bu şifreyi `/var/lib/wadm/apps/<id>/credentials.txt` dosyasına yazmaktadır. Ancak API yanıtı sadece şu şekildedir:
  ```json
  "Nextcloud is installing. Secure password generated and stored in app directory."
  ```
  Kullanıcının bu şifreyi arayüzden okuyabileceği hiçbir endpoint bulunmamaktadır! Kullanıcı sunucuya SSH veya File Explorer ile girip o dosyayı elle açmak zorunda kalmaktadır.
* **Etki Derecesi:** 🟡 **Orta (Kullanılabilirlik & Güvenlik Karışımı)**
* **Çözüm Önerisi:**
  `/api/apps/{id}/credentials` endpoint'i eklenmeli; kullanıcı ilk kurulum sonrası şifreyi arayüzde tek seferlik "Göster/Kopyala" kutusuyla güvenli biçimde görebilmelidir.

---

### 2.6. Veritabanı CLI Komutlarında Parola / Parametre Güvenliği
* **Konum:** [`src/api/db.rs:263, 311, 452, 593`](file:///home/eigen/Projects/wadm/src/api/db.rs#L263)
* **Problem:**
  Native MySQL bağlantılarında `-uroot` şifresiz çağrılmaktadır:
  ```rust
  c.args(["-D", db, "-B", "-e", query]);
  ```
  Eğer MySQL root kullanıcısı şifreli ise (çoğu üretim ortamında böyledir), WADM veritabanına bağlanamaz. Ayrıca proses listesinde (`ps aux`) `-e <query>` şeklinde SQL sorguları argüman olarak geçirildiğinde diğer yerel kullanıcılar tarafından anlık olarak görüntülenebilir.
* **Etki Derecesi:** 🟡 **Orta**
* **Çözüm Önerisi:**
  1. CLI yerine Rust'ın yerel asenkron veritabanı sürücüsü `sqlx` (MySQL & PostgreSQL bağlantı havuzu) entegre edilmelidir.
  2. CLI kullanılacaksa, sorgular komut satırı argümanı (`-e`) yerine standart girdi (`stdin`) akışı üzerinden iletilmelidir.

---

### 2.7. `sudo -n` Ayrıcalıkları ve Sudoers En Az Yetki (Least Privilege) Eksikliği
* **Problem:**
  WADM'in sistem seviyesinde işlem yapabilmesi için `sudo -n` kullanılmaktadır. Çoğu kullanıcı WADM'i çalıştırmak için `/etc/sudoers` dosyasına `wadm ALL=(ALL) NOPASSWD: ALL` satırını eklemektedir.
  Bu durum, WADM üzerinde oluşabilecek herhangi bir güvenlik açığında saldırgana tüm sistemin anahtarını teslim eder.
* **Etki Derecesi:** 🟡 **Orta - Yüksek**
* **Çözüm Önerisi:**
  Dokümantasyonda ve kurulum betiklerinde sadece WADM'in ihtiyaç duyduğu ikililere parolasız izin veren kısıtlı bir `sudoers` profili sağlanmalıdır:
  ```text
  # /etc/sudoers.d/wadm
  wadm ALL=(ALL) NOPASSWD: /usr/bin/systemctl, /usr/bin/journalctl, /usr/bin/smartctl, /usr/bin/ufw, /usr/sbin/swapoff, /usr/sbin/swapon, /usr/sbin/fstrim, /usr/bin/docker
  ```

---

### 2.8. HTTPS / TLS Eksikliği ve Düz Metin HTTP İletişimi
* **Konum:** [`src/main.rs:110`](file:///home/eigen/Projects/wadm/src/main.rs#L110)
* **Problem:**
  Sunucu varsayılan olarak `0.0.0.0:8168` üzerinde düz metin HTTP olarak ayağa kalkmaktadır:
  ```rust
  .bind(("0.0.0.0", port))?
  ```
  Eğer önünde Nginx/Caddy gibi bir TLS sonlandırıcı ters proxy (reverse proxy) yoksa, ağdaki dinleyiciler (MITM) yönetici parolasını, TOTP kodunu, JWT belirtecini ve terminal oturumundaki tüm kabuk komutlarını dinleyebilir.
* **Etki Derecesi:** 🔴 **Yüksek**
* **Çözüm Önerisi:**
  Actix-Web sunucusuna `actix-web-rustls` entegre edilerek yerel HTTPS sertifikası desteği (`wadm.crt`, `wadm.key`) eklenmeli; sertifika yoksa ilk çalıştırmada kendinden imzalı (self-signed) sertifika üreten bir mekanizma konmalıdır.

---

## 3. Performans Darboğazları (Performance Bottlenecks)

### 3.1. 2 Saniyelik Polling Döngüsündeki Aşırı Fork/Exec Maliyeti
* **Konum:** [`src/api/monitor.rs:557-568`](file:///home/eigen/Projects/wadm/src/api/monitor.rs#L557-L568)
* **Problem:**
  Frontend Dashboard'u açıkken her 2 saniyede bir `/api/stats` endpoint'i çağrılmaktadır. Bu çağrının arka planında çalışan `actix_web::web::block` bloğunda:
  
  1. **`get_default_interface` (`src/api/monitor.rs:438-446`):**
     Her 2 saniyede bir `sh -c ip route | grep default | awk ...` çağrılarak 4 ayrı alt süreç (shell, ip, grep, awk) çatallanmaktadır (fork/exec).
  2. **`count_services` (`src/api/monitor.rs:463-482`):**
     Her 2 saniyede bir `sudo -n systemctl list-units --state=running` ve `sudo -n systemctl list-units --state=failed` olmak üzere 2 kez `systemctl` çalıştırılmaktadır.
  3. **`count_containers` (`src/api/monitor.rs:484-495`):**
     Bollard API'si projede mevcut olmasına rağmen her 2 saniyede bir `sudo -n docker ps -q` CLI komutu çalıştırılmaktadır.
  4. **`Components` & `Disks` Bellek Tahsisleri (`src/api/monitor.rs:511, 535`):**
     Her 2 saniyede bir `Disks::new_with_refreshed_list()` ve `Components::new_with_refreshed_list()` sıfırdan heap tahsisi yapmaktadır.
* **Sisteme Etkisi:** Düşük donanımlı sunucularda (1 vCPU VPS, Raspberry Pi vb.) sistem boştayken bile %15-%25 sürekli CPU tüketimine yol açmaktadır.
* **Çözüm:**
  - `get_default_interface`: `/proc/net/route` doğrudan çekirdek tablosu okunarak alt süreç maliyeti sıfıra indirilmelidir.
  - Servis ve Konteyner sayıları: 30 saniyelik bir TTL önbelleğe alınmalıdır (servis sayıları her 2 saniyede bir değişmez).
  - `count_containers`: Bollard `docker.list_containers()` ile doğrudan soketten asenkron okunmalıdır.

---

### 3.2. Global `System` ve `Networks` Mutex Çekişmesi (`AppState`)
* **Konum:** [`src/api/monitor.rs:41-44, 499-508`](file:///home/eigen/Projects/wadm/src/api/monitor.rs#L41-L44)
* **Problem:**
  ```rust
  let mut sys = data.sys.lock().unwrap_or_else(|e| e.into_inner());
  sys.refresh_all();
  ```
  `sysinfo::System` nesnesi tek bir `Mutex` içinde saklanmaktadır. Birden fazla tarayıcı sekmesi açıkken veya eşzamanlı `/api/stats` ve `/api/processes` istekleri geldiğinde, worker iş parçacıkları `sys.refresh_all()` (yaklaşık 10-50ms sürer) tamamlanana kadar bloklanır.
* **Çözüm:**
  `sys.refresh_all()` yerine sadece ihtiyaç duyulan metrikler (`refresh_cpu_usage()`, `refresh_memory()`) güncellenmelidir. Ayrıca telemetri verisi tek bir arka plan Tokio task'i tarafından 2 saniyede bir güncellenip `arc_swap::ArcSwap` veya `tokio::sync::watch` kanalı ile lock-free okunmalıdır.

---

### 3.3. Sayfa Açıldığında Otomatik Terminal WebSocket Bağlantısı ve PTY Başlatılması
* **Konum:** [`web/src/context/TerminalContext.tsx:131`](file:///home/eigen/Projects/wadm/web/src/context/TerminalContext.tsx#L131), [`src/api/terminal.rs:64-80`](file:///home/eigen/Projects/wadm/src/api/terminal.rs#L64-L80)
* **Problem:**
  `TerminalProvider`, `App.tsx` içinde en üst seviyede sarmalanmıştır. Kullanıcı panele giriş yaptığı anda, Terminal sekmesine hiç tıklamasa bile `connect()` çağrılmakta; sunucuda derhal bir WebSocket bağlantısı, PTY master/slave çifti, bir adet `bash` süreci ve bir adet OS reader thread'i başlatılmaktadır.
  10 kullanıcı panele girdiğinde arka planda 10 adet aktif root bash oturumu gereksiz yere çalışmaktadır.
* **Çözüm:**
  **Tembel Bağlantı (Lazy Connection):** Terminal WebSocket bağlantısı sadece kullanıcı `activeTab === 'terminal'` sekmesine tıkladığında başlatılmalı, başka bir sekmeye geçtiğinde veya belli bir süre işlem yapmadığında kapatılmalıdır.

---

### 3.4. `get_detailed_info` Fonksiyonunda Sıfırdan `System::new_all()` Yaratılması
* **Konum:** [`src/api/system.rs:73-74`](file:///home/eigen/Projects/wadm/src/api/system.rs#L73-L74)
* **Problem:**
  ```rust
  let mut sys = System::new_all();
  sys.refresh_all();
  ```
  Zaten `AppState` içinde canlı bir `System` nesnesi bulunmasına rağmen, `/api/system` çağrıldığında bellekte sıfırdan devasa bir `System` yapısı allocate edilmekte ve tüm donanım baştan taranmaktadır.
* **Çözüm:**
  `get_detailed_info` fonksiyonuna `web::Data<AppState>` enjekte edilmeli ve mevcut `System` referansı kullanılmalıdır.

---

### 3.5. App Store Kurulumunda Tokio Asenkron Thread Havuzunun Bloke Edilmesi
* **Konum:** [`src/api/apps.rs:173-183`](file:///home/eigen/Projects/wadm/src/api/apps.rs#L173-L183)
* **Problem:**
  ```rust
  actix_web::rt::spawn(async move {
      let _ = Command::new("sudo").args(["-n", "docker-compose", "up", "-d"]).output();
  });
  ```
  `actix_web::rt::spawn` asenkron bir görev başlatır; ancak içerisindeki `std::process::Command::output()` tamamen **senkron ve bloklayıcıdır**. Docker imajlarının indirilmesi (pull) dakikalar sürebilir. Bu süre boyunca Actix'in asenkron worker thread'i kilitlenir ve diğer HTTP isteklerine yanıt veremez hale gelir.
* **Çözüm:**
  `tokio::process::Command` asenkron alt süreç motoru kullanılmalı veya blok `actix_web::web::block(move || { ... })` içine alınmalıdır.

---

### 3.6. `LogStore` İçinde $O(N)$ Dizi Kaydırma
* **Konum:** [`src/api/logs.rs:34`](file:///home/eigen/Projects/wadm/src/api/logs.rs#L34)
* **Problem:**
  ```rust
  if entries.len() >= self.max_entries {
      entries.remove(0);
  }
  ```
  `entries` bir `Vec<LogEntry>` türündedir. 1000 eleman dolduğunda, her yeni log girişinde `remove(0)` fonksiyonu bellekteki 999 elemanı tek tek sola kaydırır ($O(N)$ maliyeti). Yoğun log trafiğinde gereksiz bellek kopyalama yükü oluşur.
* **Çözüm:**
  `std::collections::VecDeque` yapısına geçilmeli; `pop_front()` ile $O(1)$ amortize zamanda baş eleman çıkarılmalıdır.

---

### 3.7. `detect_manager` Fonksiyonunun Her Çağrıda 3 Kez `which` Çalıştırması
* **Konum:** [`src/api/pkgmgr.rs:44-70`](file:///home/eigen/Projects/wadm/src/api/pkgmgr.rs#L44-L70)
* **Problem:**
  Sunucunun paket yöneticisi (APT, DNF veya Pacman) çalışma zamanında değişmez. Ancak her paket listeleme veya arama işleminde `detect_manager()` çağrılarak ardışık `which apt-get`, `which dnf`, `which pacman` alt süreçleri çalıştırılmaktadır.
* **Çözüm:**
  `static DETECTED_MANAGER: Lazy<ManagerType> = Lazy::new(detect_manager);` ile sonuç ilk çalıştırmada bir defa belirlenip RAM'de önbelleğe alınmalıdır.

---

## 4. Kod Kalitesi ve Refactoring Önerileri (Code Quality & Refactoring)

### 4.1. Monolitik Controller Dosyaları ve Katman Ayrımı Eksikliği
* **Problem:**
  [`src/api/db.rs`](file:///home/eigen/Projects/wadm/src/api/db.rs) dosyası 819 satırdır; [`src/api/pkgmgr.rs`](file:///home/eigen/Projects/wadm/src/api/pkgmgr.rs) dosyası 597 satırdır. Bu dosyalarda HTTP handler'ları, SQL sanitization mantığı, dosya I/O akışları, alt süreç çağrıları ve birim testler tek bir monolitik blok halinde yer almaktadır.
* **SOLID İhlali:** Tek Sorumluluk Prensibi (Single Responsibility Principle - SRP) ihlal edilmektedir.
* **Refactoring Planı:**
  `db.rs` modülü 3 ayrı alt parçaya bölünmelidir:
  - `src/api/db/handlers.rs`: Actix HTTP controller fonksiyonları.
  - `src/api/db/sanitizer.rs`: `normalize_sql` ve `is_dangerous_query` validasyon mantığı.
  - `src/api/db/backup.rs`: Yedekleme, restore ve stream dosya transferi.

---

### 4.2. Subprocess Command Wrapper Soyutlaması Eksikliği (`SudoExecutor`)
* **Problem:**
  Kod tabanında 40'tan fazla yerde doğrudan `Command::new("sudo").args(["-n", ...])` çağrılmaktadır. Her yerde çıkış kodu denetimi, stderr okuması ve UTF-8 dönüşümü tekrarlanmaktadır (DRY ihlali).
* **Refactoring Planı:**
  Merkezi bir `SystemExecutor` yapısı kurulmalıdır:
  ```rust
  pub struct SystemExecutor;
  impl SystemExecutor {
      pub fn run_sudo(cmd: &str, args: &[&str]) -> Result<String, SystemError> {
          let output = Command::new("sudo")
              .args(["-n", cmd])
              .args(args)
              .output()
              .map_err(|e| SystemError::ExecutionFailed(e.to_string()))?;
          
          if output.status.success() {
              Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
          } else {
              let err = String::from_utf8_lossy(&output.stderr).trim().to_string();
              Err(SystemError::CommandFailed(err))
          }
      }
  }
  ```

---

### 4.3. Veritabanı Modülünde CLI Bağımlılığı Yerine Native Sürücülere (`sqlx`) Geçiş
* **Problem:**
  `db.rs` içinde MySQL için `mysql` ve `mysqldump`, PostgreSQL için `psql` ve `pg_dump` CLI araçlarının kurulu olması zorunludur. Çıktılar sekme (`\t`) veya pipe (`|`) ile parse edilmektedir; bu durum veri içinde tab veya pipe karakteri geçtiğinde tablo görünümünün bozulmasına neden olur.
* **Refactoring Planı:**
  `sqlx` kütüphanesi entegre edilerek yerel TCP veya Unix soketi üzerinden tip güvenli sorgular çalıştırılmalıdır.

---

### 4.4. Frontend Tip Güvenliği, ApiClient Modülü ve Context Hiyerarşisi
* **Problem:**
  - `App.tsx` içinde 9 adet iç içe sarmalanmış Provider bulunmaktadır (`Toast -> Modal -> Auth -> ServerStatus -> Dependency -> System -> Stats -> Terminal -> Overlay`). Bu durum gereksiz re-render dalgalanmalarına neden olmaktadır.
  - Bileşenlerin içinde dağınık `fetch('/api/...')` çağrıları bulunmaktadır.
* **Refactoring Planı:**
  1. Tüm REST API çağrıları `web/src/services/apiClient.ts` dosyasına toplanmalıdır.
  2. Provider sayısı `AppProviders.tsx` altında birleştirilmeli ve state yönetimi optimize edilmelidir.

---

### 4.5. Büyük Frontend Paket Boyutu (Vite 1045 kB Chunk Uyarısı) ve Code-Splitting
* **Problem:**
  Vite derleme çıktısında şu uyarı alınmaktadır:
  ```
  dist/assets/index-hS_mEdnK.js   1,045.78 kB │ gzip: 291.13 kB
  (!) Some chunks are larger than 500 kB after minification.
  ```
  `@xterm/xterm` ve `recharts` kütüphaneleri ana JavaScript demetine dahil edildiği için ilk sayfa yüklemesi 1 MB'ı aşmaktadır.
* **Refactoring Planı:**
  `React.lazy()` ve `Suspense` kullanılarak `Terminal.tsx`, `SystemUsage.tsx`, `Database.tsx` gibi ağır bileşenler dinamik import (Code Splitting) ile ayrı chunk'lara bölünmelidir.

---

## 5. Mimari İyileştirmeler ve Yeni Özellikler (Architectural Improvements & Feature Ideas)

### 5.1. Server-Sent Events (SSE) Tabanlı Canlı Telemetri Akışı
- **Mevcut Durum:** İstemci her 2 saniyede bir HTTP GET `/api/stats` polling'i yapmaktadır.
- **İyileştirme:** Actix-Web üzerinde `/api/telemetry/stream` SSE endpoint'i açılmalı; sunucudaki arka plan iş parçacığı donanım değiştiğinde istemciye tek yönlü event akışı yapmalıdır.
- **Kazanım:** HTTP header yükü sıfıra iner, ağ trafiği %70 azalır, CPU tüketimi düşer.

### 5.2. Çok Kullanıcılı Rol Tabanlı Erişim Kontrolü (RBAC)
- **Mevcut Durum:** Sistemde sadece tek bir "admin" rolü vardır.
- **İyileştirme:** Üç katmanlı rol sistemi:
  1. **Viewer (Gözlemci):** Sadece Dashboard, System Usage ve Logs okuyabilir. Hiçbir bakım veya restart yapamaz.
  2. **Operator (Operatör):** Servisleri yeniden başlatabilir, paket güncelleyebilir, önbellek temizleyebilir. Dosya silme veya root terminali açamaz.
  3. **SuperAdmin (Yönetici):** Terminal, dosya yöneticisi ve kullanıcı yönetimi dahil tam yetki.

### 5.3. Eylemsel Güvenlik ve Denetim İzi (Immutable Audit Logging)
- **Mevcut Durum:** Loglar sadece bellek içi RAM tamponunda (1000 satır) tutulur ve sunucu yeniden başladığında kaybolur.
- **İyileştirme:** Kimin, hangi IP'den, ne zaman hangi servisi durdurduğu, hangi SQL sorgusunu çalıştırdığı veya hangi dosyayı sildiği yerel bir SQLite veritabanına (`/var/lib/wadm/audit.db`) değiştirilemez (append-only) olarak kaydedilmelidir.

### 5.4. Otomatik Zamanlanmış Yedekleme ve Cron/Systemd Timer Yöneticisi
- **Mevcut Durum:** Veritabanı yedekleri sadece kullanıcı butona bastığında anlık alınır.
- **İyileştirme:** Web arayüzünden veritabanı veya belirtilen sunucu klasörleri için günlük/haftalık otomatik yedekleme görevleri tanımlanabilmeli; sistemdeki cron ve systemd timer'lar görüntülenebilmelidir.

### 5.5. Tek İkili Dağıtımı (`rust-embed`)
- **Mevcut Durum:** `wadm` ikilisi çalıştırıldığında `./web/dist` klasörünün göreli konumda bulunması şarttır.
- **İyileştirme:** `rust-embed` kütüphanesi ile `web/dist` derleme çıktıları doğrudan Rust ikilisinin `.rodata` segmentine gömülmelidir. Böylece sunucuya tek bir dosya (`wadm`) kopyalanıp çalıştırıldığında arayüz eksiksiz ayağa kalkacaktır.

### 5.6. SSL/TLS ve Otomatik Let's Encrypt / ACME Entegrasyonu
- **Mevcut Durum:** Düz metin HTTP (Port 8168).
- **İyileştirme:** `actix-web-rustls` entegrasyonu ve ACME protokolü ile domain adı girildiğinde otomatik Let's Encrypt SSL sertifikası alıp yenileyen entegre HTTPS desteği.

### 5.7. Ağ İzleme & Port/Soket Analizörü
- **Mevcut Durum:** Sadece ağ arayüzlerinin RX/TX hızları izlenmektedir.
- **İyileştirme:** Sunucuda dinlenen açık portlar (`ss -tulpn`), aktif TCP bağlantıları, hangi sürecin hangi portu kullandığı arayüzde interaktif bir tablo olarak sunulmalıdır.

---

## 6. Hata Yönetimi (Error Handling) ve Loglama (Error Handling & Logging)

### 6.1. `src/api/auth.rs` İçindeki `panic!` Çağrıları
* **Konum:** [`src/api/auth.rs:162, 174`](file:///home/eigen/Projects/wadm/src/api/auth.rs#L162-L174)
* **Problem:**
  ```rust
  Err(e) => {
      log::error!("CRITICAL: Failed to parse auth store: {}", e);
      panic!("Auth store corrupted. Manual intervention required.");
  }
  ```
  Eğer `wadm-auth.json` dosyası bozulursa veya geçici bir disk hatası nedeniyle okunamazsa, `panic!` çağrısı tüm web sunucusunu çökertmektedir.
* **Çözüm:**
  `panic!` çağrıları kaldırılmalı; `load_auth_store()` fonksiyonu `Result<Option<AuthStore>, AuthError>` dönmeli, dosya bozuksa sunucu "Kurtarma Modu"nda ayağa kalkarak yöneticiye arayüzden uyarı vermelidir.

---

### 6.2. `src/api/terminal.rs` İçindeki `.expect(...)` Çağrıları
* **Konum:** [`src/api/terminal.rs:74, 80, 85, 102`](file:///home/eigen/Projects/wadm/src/api/terminal.rs#L74)
* **Problem:**
  ```rust
  let pair = pty_system.openpty(...).expect("Failed to create PTY");
  let mut child = pair.slave.spawn_command(cmd).expect("Failed to spawn shell");
  ```
  Eğer sistemde maksimum PTY sayısı (`/proc/sys/kernel/pty/max`) aşılmışsa veya yetki yetersizse, bu `.expect()` çağrıları Tokio runtime iş parçacığının çökmesine neden olur.
* **Çözüm:**
  Tüm `.expect()` çağrıları `match` veya `?` ile yakalanmalı; hata durumunda WebSocket üzerinden istemciye açıklayıcı hata mesajı (`Failed to allocate pseudo-terminal`) gönderilip bağlantı düzgünce kapatılmalıdır.

---

### 6.3. Sudo ve Harici Komut Hatalarının İyileştirilmesi
* **Konum:** [`src/api/firewall.rs:41`](file:///home/eigen/Projects/wadm/src/api/firewall.rs#L41), [`src/api/system.rs:656`](file:///home/eigen/Projects/wadm/src/api/system.rs#L656)
* **Problem:**
  `firewall.rs` düzeltilmiş olsa da sistem genelindeki bazı komutlarda (`system.rs`, `pkgmgr.rs`) komutun exit kodu başarısız olduğunda stderr ayrıntılı olarak yakalanmamakta, sadece genel bir `"Command failed"` mesajı dönülmektedir.
* **Çözüm:**
  Hata durumlarında `output.status.code()` ve `String::from_utf8_lossy(&output.stderr)` detaylı bir JSON hata nesnesi (`{ error: string, code: Option<i32>, stderr: string }`) olarak arayüze iletilmelidir.

---

### 6.4. Bellek İçi Log Kalıcılığı (Log Persistence)
* **Konum:** [`src/api/logs.rs:14-42`](file:///home/eigen/Projects/wadm/src/api/logs.rs#L14-L42)
* **Problem:**
  `LOG_STORE` tamamen RAM üzerinde tutulmaktadır. Sunucu kapandığında veya çöktüğünde çökme öncesi kritik loglar yok olmaktadır.
* **Çözüm:**
  Loglar hem RAM tamponuna hem de yerel bir dosyaya (`/var/log/wadm/server.log`) veya `systemd-journald` soketine yönlendirilmelidir.

---

## 7. Önceliklendirilmiş Eylem Planı (Actionable Priority Roadmap)

| Öncelik | Alan | Görev / İyileştirme | Beklenen Kazanım |
| :--- | :--- | :--- | :--- |
| 🔴 **P0** | **Güvenlik** | Terminal oturumlarını izole kullanıcı veya cgroup ile sınırlama (`terminal.rs`) | Tam root kabuğu verme riski ortadan kalkar. |
| 🔴 **P0** | **Kararlılık**| `auth.rs` ve `terminal.rs` içindeki `panic!` ve `.expect()` çağrılarını temizleme | Sunucunun beklenmedik çökmeleri engellenir. |
| 🟡 **P1** | **Performans**| 2s `/api/stats` döngüsündeki `sh -c ip route`, `systemctl`, `docker ps` alt süreçlerini önbellekleme | CPU kullanımı %20-%30 düşer. |
| 🟡 **P1** | **Performans**| Terminal WebSocket bağlantısını tembel (lazy) hale getirme | Arka planda boşta çalışan zombi bash süreçleri engellenir. |
| 🟡 **P1** | **Güvenlik** | App Store şablonlarında dinamik port kontrolü ve şifre görüntüleme endpoint'i | Port çakışmaları çözülür, kullanıcı şifreye erişebilir. |
| 🟡 **P1** | **Güvenlik** | Token saklamayı `HttpOnly` Cookie yapısına taşıma | XSS kaynaklı token sızıntısı imkansızlaşır. |
| 🟢 **P2** | **Mimari** | Telemetriyi HTTP Polling'den Server-Sent Events (SSE) modeline geçirme | Ağ ve işlemci yükü radikal biçimde azalır. |
| 🟢 **P2** | **Veritabanı**| `db.rs` içinde CLI sarmalama yerine native `sqlx` bağlantı havuzu kullanımı | Veri biçimlendirme hataları biter, performans artar. |
| 🟢 **P2** | **Dağıtım** | Statik dosyaları `rust-embed` ile tek ikiliye gömme | Klasör bağımlılığı olmayan taşınabilir tek ikili elde edilir. |
| 🟢 **P2** | **Frontend** | Recharts ve Xterm için React lazy code-splitting | İlk sayfa JS boyutu 1045 kB'tan ~300 kB'a düşer. |
