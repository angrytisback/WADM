# WADM (Web Administration for Linux) - Özellikler ve Yol Haritası (Features & Roadmap)

Bu belge, WADM kontrol panelinin an itibarıyla sahip olduğu tüm yetenekleri, yeni eklenecek özellikleri (Pipeline) ve gelecekte sisteme entegre edilebilecek potansiyel yetenekleri detaylandırmak amacıyla hazırlanmıştır.

## 1. Mevcut Özellikler (Current Features)

### A. Sistem İzleme (System Monitoring)
* **Gerçek Zamanlı Kaynak Tüketimi:** CPU (Çekirdek bazlı), RAM, ve Swap alanı kullanım istatistiklerinin grafiksel takibi.
* **Ağ İzleme:** Anlık ağ trafiği (Download/Upload), aktif ağ arayüzü tespiti ve teorik hız kapasitesinin gösterimi.
* **Donanım Algılama (GPU & CPU):** NVIDIA, AMD ve Intel GPU'ların tespiti; sıcaklık, VRAM ve yük metriklerinin izlenmesi. CPU paket sıcaklıklarının okunması.
* **İşlem Yöneticisi (Process Manager):** Sistemde çalışan tüm işlemlerin CPU ve RAM tüketimine göre sıralanması, SIGTERM/SIGKILL sinyalleri ile işlemlerin sonlandırılması.

### B. Paket ve Servis Yönetimi (Package & Service Management)
* **Çoklu Paket Yöneticisi Desteği:** APT (Debian/Ubuntu), DNF (RHEL/Fedora) ve Pacman (Arch Linux) entegrasyonu.
* **Paket İşlemleri:** Yüklü paketleri listeleme, yeni paket kurma, sistemdeki güncellenebilir paketleri bulma, tekil veya toplu yükseltme (upgrade) ve paket kaldırma işlemleri.
* **Servis Yönetimi (Systemd):** Aktif ve çökmüş servislerin takibi; `start`, `stop`, `restart`, `enable`, `disable` komutlarının arayüzden tetiklenmesi.
* **Servis Logları:** İlgili servis için `journalctl` üzerinden anlık log çıktılarına erişim.

### C. Docker ve Konteyner Yönetimi (Docker Management)
* **Konteyner Listesi:** Sistemdeki tüm Docker konteynerlerinin durumu, imajı ve çalışma sürelerinin takibi.
* **Konteyner Kontrolü:** Konteynerleri başlatma, durdurma, yeniden başlatma ve silme.
* **Metrikler:** Her bir konteynerin anlık CPU ve RAM kullanım detaylarına arayüzden erişim.

### D. Veritabanı Yönetimi (Database Management)
* **Çoklu Motor (Engine) Desteği:** Native ve Docker üzerinde çalışan MySQL/MariaDB ve PostgreSQL veritabanlarının tespiti.
* **Tablo ve Veri Görüntüleme:** Veritabanlarındaki tabloları ve tablo içeriklerini tarayıcı üzerinden görüntüleme.
* **SQL Çalıştırma:** Arayüzden doğrudan SQL sorguları ve mutasyonları (INSERT/UPDATE/DELETE) yollama.
* **Yedekleme ve Geri Yükleme:** Tek tıkla `mysqldump` veya `pg_dump` alarak yedek oluşturma, eski yedekleri listeleme, veritabanını önceki bir yedeğe döndürme (restore), yedek indirme (download) ve dışarıdan .sql yedeği yükleme (upload).

### E. Ağ ve Güvenlik (Network & Security)
* **Güvenli Kimlik Doğrulama:** Argon2 ile şifrelenmiş parola ve TOTP (Google Authenticator 2FA) tabanlı güvenli giriş.
* **JWT Entegrasyonu:** Her oturum için dinamik rastgele anahtarlarla (Random Secret) güvence altına alınmış token tabanlı API mimarisi.
* **Güvenlik Duvarı (UFW):** UFW durum izleme, etkinleştirme/devre dışı bırakma, yeni gelen/giden port kuralı ekleme veya silme işlemleri.
* **İnteraktif Web Terminali:** Xterm.js ve WebSocket tabanlı pty entegrasyonu ile tam teşekküllü, renkli tarayıcı terminali.

### F. Sistem Bakımı & Bellek Yönetimi (Maintenance & Memory Flush)
* **RAM / Bellek Temizleme (Memory Flush):** `sync` ve `/proc/sys/vm/drop_caches (mode 3)` ile PageCache, dentries ve inode önbelleklerini temizleme, RAM'de anında yüzlerce MB/GB boş alan açma.
* **Sistem & Paket Önbelleği Temizleme (Cache Clean):** APT, DNF veya Pacman paket yöneticilerinin indirme arşivlerini temizleme (`clean`, `autoclean`, `-Sc`), 3 günden eski systemd journal loglarını (`journalctl --vacuum-time=3d`) vakumlama ve çekirdek önbelleklerini düşürme.
* **Swap Boşaltma (Swap Flush):** Yeterli serbest RAM bulunması halinde `swapoff -a && swapon -a` ile takas alanındaki verileri güvenle fiziksel RAM'e geri aktarma (OOM korumalı).
* **SSD TRIM Protokolü:** `fstrim -av` ile SSD/NVMe disk bloklarını optimize etme ve aşınmayı azaltma.
* **Hızlı Erişim Düğmeleri (Quick Actions):** Dashboard üzerindeki System Load kartında ve System Usage Memory grafiği üzerinde tek tıkla RAM boşaltma ve önbellek temizleme kısayolları.
* **Asenkron Thread Güvenliği:** Tüm temizleme protokolleri tokio blocking thread pool (`web::block`) üzerinde çalıştırılır, Actix worker thread'lerini dondurmaz.

---

## 2. Geliştirilmekte Olan Özellikler (In-Pipeline / Requested)

Kullanıcı talebi doğrultusunda sisteme entegre edilecek yeni ana modüller şunlardır:

### A. Uygulama Mağazası & Tek Tıkla Kurulum (App Store / One-Click Apps)
* **Açıklama:** CasaOS, Umbrel veya CapRover benzeri bir deneyim sunarak, popüler Dockerize edilmiş sunucu uygulamalarının (Örn: Nextcloud, Pi-hole, Plex, WordPress, Nginx Proxy Manager) tek tıkla kurulabilmesi.
* **Özellikler:**
  * Uygulama kataloğu (App Store) arayüzü.
  * Docker Compose (veya Bollard API) üzerinden otomatik volume, network ve port atamaları.
  * Yüklenen uygulamaların güncellenmesi, silinmesi ve web arayüzlerine hızlı erişim kısayolları.

### B. Gelişmiş Dosya Yöneticisi (File Explorer & Transfer)
* **Açıklama:** Sistemdeki dosyaların tarayıcı üzerinden klasör hiyerarşisiyle gezilmesi ve düzenlenmesi.
* **Özellikler:**
  * Klasörler arası gezinme, listeleme, gizli dosyaları görme.
  * Temel operasyonlar: Klasör oluşturma, Dosya oluşturma, Yeniden adlandırma, Silme.
  * **File Editor:** Metin tabanlı dosyaları (.conf, .txt, .json) tarayıcı içinden düzenleme ve kaydetme (Monaco/Ace editör entegrasyonu).
  * **Dosya Transferi (Upload/Download):** İstemci (Client) bilgisayardan sunucuya sürükle-bırak ile dosya yükleme. Sunucudaki bir dosyayı tarayıcı üzerinden bilgisayara indirme.

### C. Depolama ve Disk Sağlığı (Storage & S.M.A.R.T Monitoring)
* **Açıklama:** SSD ve HDD disklerin kapasite durumlarına ek olarak fiziksel sağlıklarının derinlemesine izlenmesi.
* **Özellikler:**
  * `smartmontools` (smartctl) entegrasyonu ile donanımsal disk sağlığı durumu (Passed/Failed).
  * Sıcaklık, Power-On Hours, Bad Sectors (Reallocated Sector Count) gibi kritik S.M.A.R.T verilerinin okunması ve görselleştirilmesi.
  * NVMe diskler için `nvme-cli` üzerinden sağlık (Wear Leveling) göstergelerinin takibi.

### D. Ağ Hız Testi (Speedtest)
* **Açıklama:** Sunucunun dış dünya (WAN) ile olan bant genişliğini ölçme.
* **Özellikler:**
  * Ookla Speedtest CLI veya `speedtest-go` gibi araçlarla Download, Upload ve Ping/Latency sürelerinin test edilmesi.
  * Test geçmişinin kaydedilmesi ve arayüzde gösterimi.

---

## 3. Gelecek Vizyonu ve Eklenebilecek Özellikler (Future Vision & Ideas)

WADM'in tam bir "DevOps ve SysAdmin İşletim Sistemi"ne dönüşmesi için uzun vadede eklenebilecek özellikler:

1. **Reverse Proxy ve SSL Yönetimi:**
   * Nginx veya Traefik entegrasyonu ile domainleri (wadm.domain.com) yönetme.
   * Let's Encrypt / Certbot entegrasyonu ile tek tıkla SSL (HTTPS) sertifikası oluşturma ve otomatik yenileme (auto-renew).

2. **Cronjob ve Görev Zamanlayıcı:**
   * Sistemdeki periyodik görevlerin (Cron) arayüzden yönetilmesi.
   * "Her gece saat 03:00'te veritabanı yedeği al" veya "Pazar günleri sistemi güncelle" gibi zamanlanmış görevlerin (Task Scheduler) görsel olarak ayarlanması.

3. **Gelişmiş Uyarı ve Bildirim Sistemi (Alerting):**
   * CPU sıcaklığı %90'ı geçtiğinde, disk doluluğu %95'e ulaştığında veya başarısız SSH giriş denemeleri olduğunda Telegram, Discord Webhook veya E-posta üzerinden anlık bildirim (push notification) yollanması.

4. **Çoklu Kullanıcı (Multi-User) ve RBAC:**
   * WADM'e salt-okunur (read-only) kullanıcı veya sadece Docker modülüne erişebilecek kısıtlı geliştirici (dev) hesapları eklenebilmesi (Role-Based Access Control).
   * Kullanıcı işlem günlükleri (Audit Logs) ve IP takibi.

5. **Log Merkezi (Centralized Logging):**
   * `/var/log` dizinindeki Syslog, Auth log, Dmesg veya Nginx access loglarının gelişmiş filtreleme ve arama (Search) özellikleriyle akıcı bir arayüzden sunulması (Mini bir Kibana/Grafana deneyimi).

6. **Donanım Kontrolü (Fan ve Güç Yönetimi):**
   * Sensörler (lm-sensors, fancontrol) üzerinden sunucu fan hızlarının kontrol edilmesi ve güç tüketim (TDP, Watts) sınırlarının (özellikle Mini-PC ve Raspberry Pi için) ayarlanabilmesi.

7. **VPN ve Ağ Tünelleme Yönetimi:**
   * WireGuard veya Tailscale (ZeroTier) entegrasyonu. Tek tıkla sunucuyu bir VPN noktasına çevirip istemcilere QR kod ile VPN profili sunabilme.
