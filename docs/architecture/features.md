# WADM (Web Administration for Linux) - Özellikler ve Yol Haritası (Features & Roadmap)

> **Sürüm Referansı:** v0.96.0 | **Mimari Durum:** Kararlı (Production-Ready) | **Son Güncelleme:** Eylül 2026

Bu belge, WADM kontrol panelinin v0.96.0 itibarıyla sahip olduğu tüm yetenekleri, güvenlik mekanizmalarını, geliştirme hattında bulunan öncelikli mimari iyileştirmeleri (Pipeline) ve uzun vadeli vizyon hedeflerini detaylandırmak amacıyla hazırlanmış birincil özellik referansıdır.

---

## 1. Mevcut ve Aktif Özellikler (Current & Active Features)

WADM, çekirdek altyapısında harici bir runtime bağımlılığı olmaksızın derlenen **Rust (Actix-Web)** backend'i ve reaktif **React 19 / TypeScript (Vite 7)** frontend'i ile aşağıdaki modülleri tam entegre olarak sunar:

### A. Sistem İzleme ve Donanım Telemetrisi (System & Hardware Monitoring)
* **Gerçek Zamanlı Kaynak Tüketimi:**
  * Çekirdek bazlı ve paket bazlı CPU kullanım oranları ve paket sıcaklıkları (`sysinfo` ve `/sys/class/hwmon`).
  * Fiziksel RAM ve Takas (Swap) alanı kullanım detayları (Toplam, Kullanılan, Boş, Önbellek).
  * Disk bölümlerinin (partitions) dosya sistemi türü, bağlama noktası, toplam ve kullanılan alan grafikleri (`recharts` görselleştirmesi).
* **Ağ İzleme:**
  * Anlık ağ trafiği (Download/Upload RX/TX KB/s hızları) ve geçmiş telemetri çizelgesi.
  * Aktif varsayılan ağ arayüzü tespiti ve teorik hız kapasitesinin (100M/1G/10G vb.) gösterimi.
* **Hibrit Donanım & GPU Algılama:**
  * **NVIDIA:** `nvidia-smi` üzerinden VRAM kullanımı, çekirdek yükü ve sıcaklık metrikleri.
  * **AMD:** `/sys/class/drm` ve `sysfs` üzerinden `gpu_busy_percent`, VRAM ve sıcaklık okuma.
  * **Intel:** `intel_gpu_top` JSON çıktısı veya `sysfs` frekans oranı (`gt_act_freq_mhz / gt_max_freq_mhz`) ile yük tahmini.
  * Sürücüsü aktif olmayan kartlar için `/sys/bus/pci/devices` PCI sınıfı tabanlı donanım tespiti.
* **İşlem Yöneticisi (Process Manager):**
  * Sistemde çalışan tüm işlemlerin PID, kullanıcı, CPU ve RAM tüketimine göre anlık sıralanması.
  * Güvenli `SIGTERM` ve zorunlu `SIGKILL` sinyalleri ile yetkisiz/sorunlu işlemlerin sonlandırılması.
  * **Çekirdek Koruması:** Sistem init süreci (PID 1) ve WADM sunucusunun kendi ana PID'si yanlışlıkla veya kasıtlı olarak sonlandırmaya karşı korunmuştur.

### B. Paket ve Bağımlılık Yönetimi (Package & Dependency Management)
* **Çoklu Dağıtım Paket Desteği:**
  * APT (Debian, Ubuntu, Pop!_OS, Linux Mint)
  * DNF (Fedora, RHEL, AlmaLinux, Rocky Linux)
  * Pacman (Arch Linux, Manjaro, EndeavourOS)
* **Paket İşlemleri:**
  * Yüklü paketleri listeleme, filtreleme ve arama.
  * Tekil ve toplu yükseltme (`upgrade` / `update-all`).
  * Yeni paket arama ve güvenli kurulum.
  * Bağımlılık önizlemeli (dry-run) güvenli paket kaldırma (`remove-dry-run` / `remove`).
* **Sistem Bağımlılık Denetleyicisi (Dependency Health Check):**
  * WADM'in ihtiyaç duyduğu CLI araçlarının (`smartctl`, `ufw`, `fstrim`, `sensors`, `speedtest-cli`, `docker`, `mysql`, `psql`) varlığını `which` ile denetleme.
  * Eksik araçları arayüzden tek tıkla otomatik tespit edip dağıtım paket yöneticisiyle kurabilme.

### C. Servis Yönetimi (Systemd Service Management)
* **Servis Denetimi:**
  * `systemctl` entegrasyonu ile tüm sistem servislerinin listelenmesi (Active, Inactive, Failed, Dead).
  * Arayüz üzerinden tek tıkla `start`, `stop`, `restart`, `enable`, `disable` komutlarının çalıştırılması.
  * Servis adı sanitizasyonu (yalnızca geçerli karakterlere izin verilerek kabuk enjeksiyonunun engellenmesi).
* **Servis Olay Günlükleri (Service Logs):**
  * Seçili servis için `journalctl -u <service> -n 100 --no-pager` üzerinden anlık log akışına erişim.

### D. Konteyner Yönetimi (Docker Orchestration)
* **Unix Socket API İletişimi:**
  * `bollard` kütüphanesi ile `/var/run/docker.sock` üzerinden asenkron, yerel ve yüksek hızlı Docker iletişimi.
* **Konteyner Yaşam Döngüsü:**
  * Tüm konteynerlerin durum (running, exited, paused), imaj adı, port eşlemeleri ve çalışma sürelerinin takibi.
  * Başlatma, durdurma, yeniden başlatma ve silme eylemleri.
* **Canlı Konteyner Metrikleri:**
  * Her bir konteyner için gerçek zamanlı CPU yüzdesi ve bellek tüketiminin hesaplanması.

### E. Uygulama Mağazası & Tek Tıkla Kurulum (App Store / One-Click Deployments)
* **Katalog Arayüzü:** Popüler sunucu ve geliştirici servisleri için hazır şablon kataloğu:
  * **Nextcloud:** Bütünleşik MariaDB ile güvenli kişisel bulut depolama.
  * **Pi-hole:** Ağ düzeyinde DNS tabanlı reklam ve izleyici engelleme.
  * **WordPress:** MariaDB veritabanı eşliğinde blog ve içerik yönetim sistemi.
  * **Nginx Proxy Manager:** Web tabanlı SSL ve ters proxy yönetim paneli.
  * **Portainer CE:** Web tabanlı konteyner yönetim GUI'si.
* **Otomasyon ve Güvenlik:**
  * Docker Compose tabanlı otomatik volume, izole network ve port konfigürasyonu.
  * Kriptografik rastgele parolaların (`rand`) otomatik üretilip `credentials.txt` (0o600) dosyasına yazılması ve arayüzden bildirilmesi.
  * Kurulu uygulamaların durum takibi ve web arayüzlerine doğrudan yönlendirme kısayolları.

### F. Gelişmiş Dosya Yöneticisi ve Kod Editörü (File Explorer & Editor)
* **Hiyerarşik Gezinme:** Sunucu dosya sisteminde dizin ağacı, dosya boyutları, izinler (`drwxr-xr-x`) ve son değiştirilme zamanları.
* **Dosya Manipülasyonu:** Yeni dosya/klasör oluşturma, adlandırma ve silme.
* **Dahili Metin Editörü:** 10 MB'a kadar olan yapılandırma (.conf, .yaml, .json, .sh, .txt) dosyalarını tarayıcı içinde syntax formatında düzenleme ve kaydetme.
* **Dosya Transferi:**
  * **Multipart Upload:** İstemci bilgisayardan sunucu dizinine doğrudan dosya yükleme (path traversal ve ad sanitizasyonu korumalı).
  * **Download:** Dosyaları sunucudan istemciye doğrudan indirme.
* **Güvenlik Çiti (Filesystem Sandbox Guard):**
  * Path traversal (`..`) ve null bayt (`\0`) girişimlerinin engellenmesi.
  * Hassas sistem dosyalarının (`/etc/shadow`, `/etc/gshadow`, `/etc/sudoers`, `/etc/master.passwd`, `.wadm_jwt_secret`, `wadm-auth.json`) okunmasına ve `/proc`, `/sys`, `/dev`, `/etc/sudoers.d` yollarına yetkisiz yazılmasına karşı çekirdek seviyesinde koruma.

### G. Depolama ve Disk Sağlığı (Storage & S.M.A.R.T. Monitoring)
* **S.M.A.R.T. Entegrasyonu:** `smartctl --json=c` üzerinden fiziksel disklerin sağlık durumlarının incelenmesi.
* **Donanımsal Metrikler:**
  * Genel sağlık değerlendirmesi (Passed / Failed / Degraded).
  * Disk sıcaklığı, güç açık kalma süresi (Power-On Hours), yeniden ayrılan sektörler (Reallocated Sectors).
  * NVMe diskler için yıpranma oranı (Wear Leveling) ve kritik uyarı bayrakları.

### H. Ağ Hız Testi (WAN Speedtest)
* **Geniş Bant Hız Ölçümü:** Yerel `speedtest-cli` / `speedtest` aracı üzerinden sunucunun dış dünya ile olan Download, Upload hızları ve ping/gecikme sürelerinin asenkron ölçülmesi (güvensiz harici script indirmeleri engellenmiştir).
* **Sonuç Raporlama:** Dashboard ve System ekranlarında anlık test çıktısı gösterimi.

### I. Veritabanı Yönetimi (Database Management)
* **Çoklu Motor Desteği:** Yerel (native) ve Docker konteynerlerinde çalışan MySQL/MariaDB ve PostgreSQL veritabanlarının otomatik tespiti.
* **Tablo ve Veri Görüntüleme:** Şema tabloları, satır sayıları ve ilk 100 satırın veri önizlemesi.
* **SQL Konsolu:** SQL sorguları çalıştırma, mutasyon (INSERT/UPDATE/DELETE) operasyonları yürütme.
* **Yedekleme ve Geri Yükleme (Backup & Restore):**
  * `mysqldump` ve `pg_dump` CLI çağrılarının doğrudan güvenli dosya akışıyla (`Stdio::from(file)`) işletilmesi (shell redirection injection korumalı).
  * Eski yedekleri listeleme, geri yükleme (restore), istemciye indirme ve `.sql` yedeği yükleme.

### J. Sistem Bakımı ve Bellek Yönetimi (Maintenance Protocols)
* **RAM Boşaltma (Memory Flush):** `sync` ve `/proc/sys/vm/drop_caches` (mode 3) ile PageCache, dentries ve inode önbelleklerini anında boşaltma.
* **Sistem Önbelleği Temizleme (Cache Clean):** Paket yöneticisi arşivlerini (`apt clean`, `pacman -Sc`, `dnf clean all`) temizleme, 3 günden eski systemd journal kayıtlarını vakumlama (`journalctl --vacuum-time=3d`).
* **Swap Tahliyesi (Swap Flush):** Yeterli serbest RAM bulunması halinde `swapoff -a && swapon -a` ile takas alanını fiziksel RAM'e aktarma (OOM korumalı eşik kontrolü: `avail_kb >= swap_used + 150MB`).
* **SSD TRIM:** `fstrim -av` ile SSD/NVMe bloklarını optimize etme.
* **DNS Önbellek Temizliği:** `resolvectl flush-caches` / `systemd-resolve` desteği.

### K. Güvenlik, Kimlik Doğrulama ve Ağ Koruma (Security & Authentication)
* **Kriptografik Kimlik Doğrulama:**
  * Argon2id algoritmasıyla hash'lenmiş yönetici parolası.
  * RFC 6238 uyumlu TOTP (Google Authenticator / Aegis) 2FA zorunluluğu (SHA256 ve SHA1 uyumlu).
  * Çalışma zamanında üretilen dinamik 256-bit JWT secret (`.wadm_jwt_secret`) ve 0o600 dosya izinleri.
* **Kaba Kuvvet (Brute-Force) Koruması:** `LoginRateLimiter` ile IP bazında 60 saniyede en fazla 5 başarısız giriş denemesi kuralı.
* **Güvenlik Duvarı (UFW):**
  * UFW aktif/pasif durum kontrolü (`--force` ile güvenli işletim).
  * Gelen/Giden port kurallarını listeleme, yeni port/protokol kuralı ekleme veya silme (`is_safe_ufw_rule` komut enjeksiyon korumalı).
* **Güvenlik Başlıkları & CORS:**
  * `X-Content-Type-Options: nosniff`, `X-Frame-Options: DENY`, `Referrer-Policy: strict-origin-when-cross-origin`, `Permissions-Policy: geolocation=(), microphone=(), camera=()`.
  * Ortam değişkeni tabanlı (`WADM_ALLOWED_ORIGINS`) veya yerel origin kısıtlamalı dinamik CORS koruması.
* **İstemci Token İzolasyonu:** `AuthContext.tsx` interceptor'ında `isInternalOrRelative` kontrolü ile token'ın harici URL'lere sızması engellenmiştir.

### L. İnteraktif Web Terminali (Web TTY)
* **Bütünleşik Terminal Deneyimi:**
  * `@xterm/xterm` (v6.x) ve `@xterm/addon-fit` ile yüksek performanslı tarayıcı konsolu.
  * `portable-pty` ve Actix WebSocket (`actix-ws`) üzerinden tam teşekküllü interaktif bash oturumu.
  * Dinamik terminal yeniden boyutlandırma (`RESIZE:colsxrows`).
  * Kimlik doğrulaması: Token, WebSocket alt protokolü (`Sec-WebSocket-Protocol`) üzerinden taşınır.
  * Geliştirici Modu (`developer_mode`) ile yetki denetimi.

### M. Güç Yönetimi (Power Management & Sequencing)
* **Donanımsal Güç Kontrolü:**
  * Anlık yeniden başlatma (`reboot`) ve anlık güvenli kapatma (`shutdown -h now`).
  * `systemd-logind` entegrasyonu ile dakika hassasiyetinde zamanlanmış kapatma/yeniden başlatma planlama ve planı iptal etme (`shutdown -c`).

### N. Olay Günlüğü ve İzleme (System Event Logging)
* **Dahili Loglama:** `LogStore` halka tamponu ile son 1000 sistem işleminin seviyesi (INFO/WARN/ERROR), zaman damgası ve mesajıyla in-memory saklanması.
* **Arayüzden İnceleme:** WADM sistem loglarını arayüzden inceleme, filtreleme ve tek tıkla temizleme.

---

## 2. Geliştirilmekte Olan Özellikler ve Öncelikli Yol Haritası (In-Pipeline & Roadmap)

Sistem analizi, güvenlik denetimi ve mimari refactoring planı doğrultusunda geliştirme hattında bulunan öncelikli hedefler:

### A. Server-Sent Events (SSE) / WebSocket Canlı Telemetri Akışı
* **Problem:** Mevcut HTTP polling yaklaşımı (2 saniyede bir GET `/api/stats`) gereksiz TCP el sıkışması ve Mutex kilitlenmesine neden olmaktadır.
* **Hedef:** Arka planda çalışan bir Tokio broadcast kanalı üzerinden `/api/telemetry/stream` (SSE) uç noktası açılarak verinin tek bağlantı üzerinden istemciye akıtılması; ağ yükünün %80 oranında düşürülmesi.

### B. Paket Güncelleme Sayımı için Akıllı Asenkron Önbellek Motoru (Package Cache Engine)
* **Problem:** Her 2 saniyelik stats çağrısında senkron `apt list --upgradable` komutunun tetiklenmesi CPU ve disk I/O darboğazına yol açmaktadır.
* **Hedef:** Güncellenebilir paket sayısının arka planda 30 dakikalık TTL ile asenkron önbelleğe alınması (`UPGRADABLE_CACHE`) ve kullanıcı paket sayfasına girdiğinde önbelleğin geçersiz kılınması.

### C. RFC 7807 Uyumlu Hata Yönetimi (`WadmError`) ve Panic Temizliği
* **Problem:** Çeşitli controller'larda bulunan `panic!` ve `expect()` çağrıları, beklenmedik dosya bozulmalarında tüm web sunucusunu çökertebilmektedir. Hata JSON formatları tutarsızdır.
* **Hedef:** `thiserror` tabanlı `WadmError` enum'ının oluşturulması, `actix_web::ResponseError` trait'i ile RFC 7807 uyumlu JSON hata yanıtlarının (`title`, `status`, `detail`) dönülmesi ve tüm panic'lerin kurtarılabilir `Result` modellerine dönüştürülmesi.

### D. İki Aşamalı Güvenli Dosya İndirme Protokolü (Blob Download / Ticket Exchange)
* **Problem:** `FileExplorer.tsx` içindeki `window.open` indirmesi Authorization başlığı taşıyamadığı için 401 hatası vermektedir.
* **Hedef:** Frontend indirme işleminin `fetch` ile Authorization başlığı eşliğinde blob formatında tetiklenmesi veya sunucu tarafında 30 saniye geçerli tek kullanımlık indirme bileti (`download_ticket`) mekanizması kurulması.

### E. Gelişmiş PTY Yaşam Döngüsü ve Zombi Süreç Temizleyici (`Child Process Reaper`)
* **Problem:** Terminal WebSocket bağlantısı koptuğunda arka plandaki `bash` süreci açık kalabilmekte ve PID limitlerini zorlayabilmektedir.
* **Hedef:** `portable_pty::Child` nesnesinin bağlantı kapanışında (`AggregatedMessage::Close` veya kanal kopması) zorunlu olarak `kill()` ve `wait()` ile sonlandırılması.

### F. Trait Tabanlı Modüler Mimari (`PackageManager` & `GpuProvider`)
* **Problem:** `pkgmgr.rs` ve `monitor.rs` içerisindeki uzun `match` blokları Single Responsibility (SOLID) prensibini ihlal etmektedir.
* **Hedef:** `PackageManager` ve `GpuProvider` trait soyutlamaları yapılarak kod tabanının dağıtım ve donanım bazında izole dosyalara bölünmesi (`monitor/gpu/nvidia.rs`, `pkgmgr/apt.rs` vb.).

### G. Doğrudan Asenkron Veritabanı Sürücü Entegrasyonu (`sqlx`)
* **Problem:** MySQL ve PostgreSQL işlemlerinin CLI sarmalayarak (`mysql -e`, `psql -c`) ve stdout ayrıştırarak yapılması veri bozulmalarına ve enjeksiyon risklerine açıktır.
* **Hedef:** `sqlx` kütüphanesiyle asenkron bağlantı havuzu (connection pool) oluşturulması, doğrudan TCP/Socket protokolü üzerinden parametrik sorguların çalıştırılması.

---

## 3. Gelecek Vizyonu ve Eklenebilecek Genişleme Modülleri (Future Vision)

WADM'in uçtan uca modern bir "DevOps ve SysAdmin İşletim Sistemi" standardına ulaşması için uzun vadeli vizyon hedefleri:

1. **Ters Proxy (Reverse Proxy) ve Otomatik SSL Yönetimi:**
   * Entegre Nginx veya Caddy yönetim arayüzü ile domain yönlendirmeleri (`app.domain.com -> 127.0.0.1:3000`).
   * Let's Encrypt / ACME protokolü ile tek tıkla SSL sertifikası üretme ve otomatik yenileme (auto-renew).

2. **Görsel Görev Zamanlayıcı (Cronjob & Task Scheduler):**
   * Linux `crontab` ve systemd timers yapılandırmalarının arayüzden yönetilmesi.
   * "Her gece saat 03:00'te veritabanı yedeği al", "Her Pazar SSD TRIM çalıştır" gibi zamanlanmış görevlerin tanımlanabilmesi.

3. **Çoklu Kullanıcı Desteği ve Rol Tabanlı Yetkilendirme (RBAC & Audit Logs):**
   * SQLite tabanlı yerel denetim günlüğü (`audit.db`) ile kullanıcı işlemlerinin (hangi IP, hangi saatte, hangi servisi durdurdu) kaydedilmesi.
   * `Viewer` (salt-okunur telemetri), `Operator` (konteyner ve servis yönetimi) ve `SuperAdmin` (tam yetkili) rol ayrımı.

4. **Gelişmiş Alarm ve Bildirim Motoru (Alerting & Webhooks):**
   * CPU sıcaklığı eşiği aşıldığında, disk doluluğu %90'a ulaştığında veya başarısız giriş denemeleri tespit edildiğinde Telegram, Discord Webhook, Slack veya E-posta üzerinden anlık push bildirim gönderimi.

5. **Donanım Fan ve Güç Sınırı Yönetimi (Hardware Fan & TDP Control):**
   * `lm-sensors` ve `fancontrol` entegrasyonu ile sunucu fan devir profillerinin (Silent, Balanced, Performance) ayarlanabilmesi.
   * Özellikle Mini-PC ve Raspberry Pi donanımlarında CPU frekans ve güç sınırlarının (TDP) sınırlandırılabilmesi.

6. **VPN ve Ağ Tünelleme Yönetimi (WireGuard & Tailscale):**
   * WireGuard çekirdek modülü veya Tailscale/ZeroTier istemcisi entegrasyonu.
   * Sunucuyu tek tıkla güvenli VPN ağ geçidine dönüştürme ve istemcilere QR kod ile VPN profili sunma.

---

## 4. Sistem Uyumluluk ve Destek Matrisi (Compatibility Matrix)

| Bileşen / Özellik | Debian / Ubuntu | Fedora / RHEL | Arch Linux | Destek Durumu |
| :--- | :---: | :---: | :---: | :---: |
| **Paket Yöneticisi** | APT (`apt-get`) | DNF (`dnf`) | Pacman (`pacman`) | ✅ Tam Destekli |
| **Servis Yönetimi** | Systemd (`systemctl`) | Systemd (`systemctl`) | Systemd (`systemctl`) | ✅ Tam Destekli |
| **Konteyner Yönetimi** | Docker Engine | Docker Engine / Podman* | Docker Engine | ✅ Tam Destekli (*Docker socket) |
| **Veritabanı Yönetimi** | MySQL / PostgreSQL | MySQL / PostgreSQL | MySQL / PostgreSQL | ✅ Native ve Docker |
| **Web Terminali** | Portable-PTY (bash) | Portable-PTY (bash) | Portable-PTY (bash) | ✅ Tam Destekli |
| **Depolama Sağlığı** | `smartmontools` | `smartmontools` | `smartmontools` | ✅ Tam Destekli |
| **Güvenlik Duvarı** | UFW | UFW / Firewalld* | UFW | ✅ UFW Destekli (*Firewalld planlanıyor) |
| **Donanım Algılama** | NVIDIA / AMD / Intel | NVIDIA / AMD / Intel | NVIDIA / AMD / Intel | ✅ Hibrit sysfs + CLI |
| **Ağ Hız Testi** | `speedtest-cli` | `speedtest-cli` | `speedtest-cli` | ✅ Asenkron CLI |
| **Güç Yönetimi** | `systemd-logind` | `systemd-logind` | `systemd-logind` | ✅ Tam Destekli |\n