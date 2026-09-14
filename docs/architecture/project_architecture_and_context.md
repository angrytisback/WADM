# WADM (Web Administration for Linux) - Proje Mimarisi ve Bağlam Haritası

## 1. Sistem Özeti
WADM, Linux sistemleri yönetmek, izlemek ve yapılandırmak için geliştirilmiş olan, modern (glassmorphism UI) ve hafif bir web tabanlı kontrol panelidir. 
Uygulama, temel olarak sistem kaynaklarını (CPU, RAM, Disk, Ağ, GPU) takip etmeye, docker konteynerlerini, servisleri (systemd), paket yöneticilerini (apt, dnf, pacman) ve veri tabanlarını (MySQL, Postgres) yönetmeye olanak tanır.
Bütünleşik bir terminal deneyimi ve güvenlik odaklı yapı (TOTP tabanlı 2FA kimlik doğrulama) sunar.

## 2. Mimari Tasarım ve Veri Akışı
Sistem "İstemci-Sunucu" (Client-Server) mimarisinde çalışır:
* **Frontend (İstemci):** React (Vite, TypeScript) tabanlı Tek Sayfa Uygulaması (SPA) olarak tasarlanmıştır. Veriler `context` API üzerinden modüller arası paylaşılır.
* **Backend (Sunucu):** Rust ve `actix-web` framework'ü ile asenkron bir HTTP sunucusu olarak çalışır. Gelen istekler API üzerinden işlenir veya WebSocket (terminal vb. interaktif işlemler için) ile stream edilir.
* **Veri Akışı:** İstemci, bir eylem başlattığında (örneğin servis başlatma), backend bu isteği alır, `sudo` veya doğrudan sistem çağrıları (Command::new) aracılığıyla Linux işletim sisteminde ilgili aracı tetikler ve stdout/stderr çıktılarını formatlayıp istemciye geri döner.

## 3. Modül ve Dosya Haritası

### Kök Dizin (Root)
* `Cargo.toml` & `Cargo.lock`: Rust backend projesinin bağımlılıkları ve konfigürasyonları.
* `web/`: Frontend uygulamasının kök dizini (React).
* `compile.sh` / `compile_all.sh`: Dağıtım ve platform bazlı derleme süreçlerini otomatize eden scriptler.

### Frontend (`web/src/`)
* `App.tsx`: Tüm provider'ların (Auth, System, Stats vb.) entegre edildiği ana entry point ve sidebar/router mekanizması.
* `components/`: Arayüzü oluşturan ekranlar.
  * `Dashboard.tsx`, `SystemUsage.tsx`: Sistem metriklerinin görselleştirildiği paneller.
  * `Packages.tsx`, `Services.tsx`, `Firewall.tsx`, `Docker.tsx`: Yönetim bileşenleri.
  * `Terminal.tsx`: Xterm.js kullanılarak tarayıcı üzerinde pty entegrasyonu sunan bileşen.
* `context/`: State yönetimi için kullanılan context'ler (Ör. `AuthContext.tsx`, `SystemContext.tsx`).

### Backend (`src/`)
* `main.rs`: Uygulamanın başlangıç noktası. Global statelerin (Auth store, sysinfo), Logger'ın başlatılması ve HTTP rotalarının (Router) yapılandırılması. Frontend çıktısı (`web/dist`) statik olarak sunulur.
* `middleware.rs`: İstekleri yakalayıp JWT yetkilendirme kontrollerini gerçekleştiren Actix middleware bileşeni (`Auth` transform).
* `api/`: Sistemin tüm iş mantığı (Controllers):
  * `auth.rs`: Argon2 ile parola hashi doğrulama, TOTP (2FA) süreçleri ve JWT token üretimi (`wadm-auth.json` dosyasına veri persist edilir).
  * `config.rs` & `dependencies.rs`: Uygulama ve sistem bağımlılık konfigürasyonları.
  * `db.rs`: Native ve Dockerize edilmiş MySQL/PostgreSQL veritabanlarının sorgulanması, dump alınması ve yedeğe dönülmesi.
  * `docker.rs`: Bollard kütüphanesi ile Docker daemon ile haberleşme.
  * `monitor.rs`: İşletim sistemindeki metrikleri (CPU, GPU, RAM) `sysinfo` ve hwmon dizinlerini tarayarak (Nvidia-SMI/Intel_GPU_Top) sağlayan donanım izleme mekanizması.
  * `pkgmgr.rs`: APT, DNF ve Pacman paket yöneticilerinin sarmalanması (Wrap).
  * `services.rs`: `systemctl` ve `journalctl` üzerinden servis yönetimi ve log okuma.
  * `terminal.rs`: `portable-pty` kütüphanesi kullanılarak arka planda TTY (pseudo-terminal) açılması ve WebSocket üzerinden tarayıcı ile IO akışı kurulması.

## 4. Temel Algoritmalar ve State Yönetimi
* **Global Monitoring State:** Rust tarafında `AppState` adında global bir struct bulunur. Bu yapı içinde `Mutex<System>` ve `Mutex<Networks>` tutulur. Frontend periyodik olarak `/api/stats` uç noktasını çağırır. Her çağrıda bu mutex kilitlenir, `sys.refresh_all()` ile yenilenir ve donanım istatistikleri sunulur.
* **GPU Keşif ve Metrik (Heuristic Discovery):** `monitor.rs` içerisindeki `get_gpu_stats`, `/sys/class/drm` ve pci id'lerini tarayarak vendor tespiti yapar. NVIDIA için `nvidia-smi`, Intel için `intel_gpu_top` veya native donanım frekans metrikleri gibi birden farklı algoritma ile izleme sağlar.
* **WebSocket TTY Loop:** Kullanıcı terminal oturumu açtığında, Actix thread'i üzerinde asenkron bir `tokio::select!` döngüsü başlar. Master/Slave PTY ile soket arasına buffer köprülenir; bir yandan process'in çıktısı okunur (reader.read), diğer yandan tarayıcıdan gelen girdiler (writer.write_all) sisteme enjekte edilir.

## 5. Bağımlılıklar ve Entegrasyonlar
* **Actix-Web & Actix-WS:** Yüksek performanslı asenkron web ve websocket framework'ü.
* **Sysinfo:** Donanım bazlı düşük seviyeli kaynak ölçümleri.
* **Bollard:** Docker API entegrasyonu.
* **Totp-RS & Argon2:** 2FA kod doğrulama ve güvenli şifre saklama standartları.
* **Xterm.js (Frontend):** Tarayıcıda tam uyumlu bir xterm emülatörü sağlamak amacıyla kullanılır.
