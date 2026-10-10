use actix_web::{web, HttpResponse, Responder};
use chrono::Utc;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::{Arc, Mutex};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::sync::{broadcast, mpsc};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Job {
    pub id: String,
    pub job_type: String,
    pub status: String, // "pending", "running", "completed", "failed"
    pub progress: i32,  // 0-100 or -1
    pub logs: String,
    pub error_message: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobSummary {
    pub id: String,
    pub job_type: String,
    pub status: String,
    pub progress: i32,
    pub error_message: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobActionResponse {
    pub job_id: String,
    pub status: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct JobLogEvent {
    pub job_id: String,
    pub event_type: String, // "log", "status", "complete", "error"
    pub line: String,
    pub status: String,
    pub progress: i32,
    pub timestamp: String,
}

#[allow(dead_code)]
pub enum JobTaskPayload {
    PackageUpdateAll {
        manager: String,
    },
    DbBackup {
        engine: String,
        db: String,
        container_id: Option<String>,
        target_path: PathBuf,
    },
    AppInstall {
        app_id: String,
        app_name: String,
        app_dir: PathBuf,
        _secure_pwd: String,
    },
    CommandExecution {
        cmd: String,
        args: Vec<String>,
        output_file: Option<PathBuf>,
        description: String,
    },
    AcmeIssueCertificate {
        domain: String,
        email: String,
    },
    PluginInstall {
        plugin_id: String,
        download_url: String,
        expected_sha256: String,
        manager: Arc<crate::plugins::PluginManager>,
    },
    PluginUninstall {
        plugin_id: String,
        manager: Arc<crate::plugins::PluginManager>,
    },
}

pub struct JobTask {
    pub id: String,
    pub job_type: String,
    pub payload: JobTaskPayload,
}

pub struct JobDatabase {
    conn: Mutex<Connection>,
}

impl JobDatabase {
    pub fn new(path: PathBuf) -> Result<Self, rusqlite::Error> {
        let conn = Connection::open(&path)?;
        conn.execute_batch(
            "PRAGMA journal_mode = WAL;
             PRAGMA synchronous = NORMAL;
             CREATE TABLE IF NOT EXISTS jobs (
                 id TEXT PRIMARY KEY,
                 job_type TEXT NOT NULL,
                 status TEXT NOT NULL,
                 progress INTEGER NOT NULL DEFAULT 0,
                 logs TEXT NOT NULL DEFAULT '',
                 error_message TEXT,
                 created_at TEXT NOT NULL,
                 updated_at TEXT NOT NULL
             );
             CREATE INDEX IF NOT EXISTS idx_jobs_created_at ON jobs(created_at DESC);",
        )?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    #[cfg(test)]
    pub fn new_in_memory() -> Result<Self, rusqlite::Error> {
        let conn = Connection::open_in_memory()?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS jobs (
                 id TEXT PRIMARY KEY,
                 job_type TEXT NOT NULL,
                 status TEXT NOT NULL,
                 progress INTEGER NOT NULL DEFAULT 0,
                 logs TEXT NOT NULL DEFAULT '',
                 error_message TEXT,
                 created_at TEXT NOT NULL,
                 updated_at TEXT NOT NULL
             );",
        )?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    pub fn create_job(&self, id: &str, job_type: &str) -> Result<Job, rusqlite::Error> {
        let now = Utc::now().to_rfc3339();
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO jobs (id, job_type, status, progress, logs, error_message, created_at, updated_at)
             VALUES (?1, ?2, 'pending', 0, '', NULL, ?3, ?3)",
            params![id, job_type, now],
        )?;

        Ok(Job {
            id: id.to_string(),
            job_type: job_type.to_string(),
            status: "pending".to_string(),
            progress: 0,
            logs: String::new(),
            error_message: None,
            created_at: now.clone(),
            updated_at: now,
        })
    }

    pub fn update_status(
        &self,
        id: &str,
        status: &str,
        progress: i32,
        error_message: Option<&str>,
    ) -> Result<(), rusqlite::Error> {
        let now = Utc::now().to_rfc3339();
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE jobs SET status = ?1, progress = ?2, error_message = ?3, updated_at = ?4 WHERE id = ?5",
            params![status, progress, error_message, now, id],
        )?;
        Ok(())
    }

    pub fn append_log(&self, id: &str, line: &str) -> Result<(), rusqlite::Error> {
        let now = Utc::now().to_rfc3339();
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE jobs SET logs = logs || ?1 || char(10), updated_at = ?2 WHERE id = ?3",
            params![line, now, id],
        )?;
        Ok(())
    }

    pub fn get_job(&self, id: &str) -> Result<Option<Job>, rusqlite::Error> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, job_type, status, progress, logs, error_message, created_at, updated_at
             FROM jobs WHERE id = ?1",
        )?;

        let mut rows = stmt.query(params![id])?;
        if let Some(row) = rows.next()? {
            Ok(Some(Job {
                id: row.get(0)?,
                job_type: row.get(1)?,
                status: row.get(2)?,
                progress: row.get(3)?,
                logs: row.get(4)?,
                error_message: row.get(5)?,
                created_at: row.get(6)?,
                updated_at: row.get(7)?,
            }))
        } else {
            Ok(None)
        }
    }

    pub fn list_jobs(&self, limit: usize) -> Result<Vec<JobSummary>, rusqlite::Error> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, job_type, status, progress, error_message, created_at, updated_at
             FROM jobs ORDER BY created_at DESC LIMIT ?1",
        )?;

        let rows = stmt.query_map(params![limit as i64], |row| {
            Ok(JobSummary {
                id: row.get(0)?,
                job_type: row.get(1)?,
                status: row.get(2)?,
                progress: row.get(3)?,
                error_message: row.get(4)?,
                created_at: row.get(5)?,
                updated_at: row.get(6)?,
            })
        })?;

        let mut list = Vec::new();
        for r in rows {
            list.push(r?);
        }
        Ok(list)
    }
}

pub struct JobManager {
    db: Arc<JobDatabase>,
    task_tx: mpsc::Sender<JobTask>,
    broadcasters: Arc<Mutex<HashMap<String, broadcast::Sender<JobLogEvent>>>>,
}

impl JobManager {
    pub fn new_with_ssl(
        db: Arc<JobDatabase>,
        challenge_store: crate::ssl::AcmeChallengeStore,
        ssl_db: Arc<crate::ssl::SslDatabase>,
    ) -> Arc<Self> {
        let (task_tx, task_rx) = mpsc::channel::<JobTask>(64);
        let broadcasters = Arc::new(Mutex::new(HashMap::new()));

        let manager = Arc::new(Self {
            db: db.clone(),
            task_tx,
            broadcasters: broadcasters.clone(),
        });

        // Spawn Tokio worker daemon with SSL context
        Self::start_worker(
            task_rx,
            db,
            broadcasters,
            Some(challenge_store),
            Some(ssl_db),
        );

        manager
    }

    #[allow(dead_code)]
    pub fn new(db: Arc<JobDatabase>) -> Arc<Self> {
        let (task_tx, task_rx) = mpsc::channel::<JobTask>(64);
        let broadcasters = Arc::new(Mutex::new(HashMap::new()));

        let manager = Arc::new(Self {
            db: db.clone(),
            task_tx,
            broadcasters: broadcasters.clone(),
        });

        // Spawn Tokio worker daemon
        Self::start_worker(task_rx, db, broadcasters, None, None);

        manager
    }

    fn get_or_create_broadcaster(
        broadcasters: &Arc<Mutex<HashMap<String, broadcast::Sender<JobLogEvent>>>>,
        job_id: &str,
    ) -> broadcast::Sender<JobLogEvent> {
        let mut map = broadcasters.lock().unwrap();
        if let Some(tx) = map.get(job_id) {
            tx.clone()
        } else {
            let (tx, _) = broadcast::channel(256);
            map.insert(job_id.to_string(), tx.clone());
            tx
        }
    }

    fn start_worker(
        mut rx: mpsc::Receiver<JobTask>,
        db: Arc<JobDatabase>,
        broadcasters: Arc<Mutex<HashMap<String, broadcast::Sender<JobLogEvent>>>>,
        challenge_store: Option<crate::ssl::AcmeChallengeStore>,
        ssl_db: Option<Arc<crate::ssl::SslDatabase>>,
    ) {
        tokio::spawn(async move {
            log::info!("JobManager worker engine started and ready to process jobs.");
            while let Some(task) = rx.recv().await {
                let job_id = task.id.clone();
                let job_type = task.job_type.clone();
                let db_clone = db.clone();
                let tx_log = Self::get_or_create_broadcaster(&broadcasters, &job_id);

                log::info!("Processing job {} [{}]", job_id, job_type);
                let _ = db_clone.update_status(&job_id, "running", 5, None);
                let _ = tx_log.send(JobLogEvent {
                    job_id: job_id.clone(),
                    event_type: "status".to_string(),
                    line: format!("Job {} started ({})", job_id, job_type),
                    status: "running".to_string(),
                    progress: 5,
                    timestamp: Utc::now().to_rfc3339(),
                });

                let result = match task.payload {
                    JobTaskPayload::PackageUpdateAll { manager } => {
                        Self::execute_package_update(&job_id, &manager, &db_clone, &tx_log).await
                    }
                    JobTaskPayload::DbBackup {
                        engine,
                        db,
                        container_id,
                        target_path,
                    } => {
                        Self::execute_db_backup(
                            &job_id,
                            &engine,
                            &db,
                            container_id.as_deref(),
                            &target_path,
                            &db_clone,
                            &tx_log,
                        )
                        .await
                    }
                    JobTaskPayload::AppInstall {
                        app_id,
                        app_name,
                        app_dir,
                        _secure_pwd: _,
                    } => {
                        Self::execute_app_install(
                            &job_id, &app_id, &app_name, &app_dir, &db_clone, &tx_log,
                        )
                        .await
                    }
                    JobTaskPayload::CommandExecution {
                        cmd,
                        args,
                        output_file,
                        description,
                    } => {
                        Self::execute_command_job(
                            &job_id,
                            &cmd,
                            &args,
                            output_file.as_ref(),
                            &description,
                            &db_clone,
                            &tx_log,
                        )
                        .await
                    }
                    JobTaskPayload::AcmeIssueCertificate { domain, email } => {
                        if let (Some(cs), Some(s_db)) = (challenge_store.as_ref(), ssl_db.as_ref())
                        {
                            Self::execute_acme_issue(
                                &job_id, &domain, &email, cs, s_db, &db_clone, &tx_log,
                            )
                            .await
                        } else {
                            Err("SSL context not initialized in JobManager".to_string())
                        }
                    }
                    JobTaskPayload::PluginInstall {
                        plugin_id,
                        download_url,
                        expected_sha256,
                        manager,
                    } => {
                        crate::plugins::installer::install_plugin(
                            &job_id,
                            &plugin_id,
                            &download_url,
                            &expected_sha256,
                            &db_clone,
                            &tx_log,
                            &manager,
                        )
                        .await
                    }
                    JobTaskPayload::PluginUninstall { plugin_id, manager } => {
                        crate::plugins::installer::uninstall_plugin(
                            &job_id, &plugin_id, &db_clone, &tx_log, &manager,
                        )
                        .await
                    }
                };

                match result {
                    Ok(final_msg) => {
                        let _ = db_clone.update_status(&job_id, "completed", 100, None);
                        let _ = tx_log.send(JobLogEvent {
                            job_id: job_id.clone(),
                            event_type: "complete".to_string(),
                            line: final_msg,
                            status: "completed".to_string(),
                            progress: 100,
                            timestamp: Utc::now().to_rfc3339(),
                        });
                        log::info!("Job {} completed successfully", job_id);
                    }
                    Err(err_msg) => {
                        let _ = db_clone.update_status(&job_id, "failed", -1, Some(&err_msg));
                        let _ = tx_log.send(JobLogEvent {
                            job_id: job_id.clone(),
                            event_type: "error".to_string(),
                            line: format!("Error: {}", err_msg),
                            status: "failed".to_string(),
                            progress: -1,
                            timestamp: Utc::now().to_rfc3339(),
                        });
                        log::error!("Job {} failed: {}", job_id, err_msg);
                    }
                }
            }
        });
    }

    async fn stream_command_output(
        job_id: &str,
        mut cmd: Command,
        db: &Arc<JobDatabase>,
        tx_log: &broadcast::Sender<JobLogEvent>,
        current_progress: i32,
    ) -> Result<(), String> {
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        let mut child = cmd
            .spawn()
            .map_err(|e| format!("Failed to spawn command: {}", e))?;

        let stdout = child.stdout.take();
        let stderr = child.stderr.take();

        let mut tasks = Vec::new();

        if let Some(stdout) = stdout {
            let db_c = db.clone();
            let tx_c = tx_log.clone();
            let jid = job_id.to_string();
            tasks.push(tokio::spawn(async move {
                let reader = BufReader::new(stdout);
                let mut lines = reader.lines();
                while let Ok(Some(line)) = lines.next_line().await {
                    let _ = db_c.append_log(&jid, &line);
                    let _ = tx_c.send(JobLogEvent {
                        job_id: jid.clone(),
                        event_type: "log".to_string(),
                        line,
                        status: "running".to_string(),
                        progress: current_progress,
                        timestamp: Utc::now().to_rfc3339(),
                    });
                }
            }));
        }

        if let Some(stderr) = stderr {
            let db_c = db.clone();
            let tx_c = tx_log.clone();
            let jid = job_id.to_string();
            tasks.push(tokio::spawn(async move {
                let reader = BufReader::new(stderr);
                let mut lines = reader.lines();
                while let Ok(Some(line)) = lines.next_line().await {
                    let _ = db_c.append_log(&jid, &line);
                    let _ = tx_c.send(JobLogEvent {
                        job_id: jid.clone(),
                        event_type: "log".to_string(),
                        line,
                        status: "running".to_string(),
                        progress: current_progress,
                        timestamp: Utc::now().to_rfc3339(),
                    });
                }
            }));
        }

        for t in tasks {
            let _ = t.await;
        }

        let status = child
            .wait()
            .await
            .map_err(|e| format!("Process wait failed: {}", e))?;

        if status.success() {
            Ok(())
        } else {
            Err(format!(
                "Command exited with non-zero status: {:?}",
                status.code()
            ))
        }
    }

    async fn execute_package_update(
        job_id: &str,
        manager: &str,
        db: &Arc<JobDatabase>,
        tx_log: &broadcast::Sender<JobLogEvent>,
    ) -> Result<String, String> {
        let _ = db.append_log(
            job_id,
            &format!("--- Starting package update for manager: {} ---", manager),
        );
        let _ = tx_log.send(JobLogEvent {
            job_id: job_id.to_string(),
            event_type: "log".to_string(),
            line: format!("--- Starting package update for manager: {} ---", manager),
            status: "running".to_string(),
            progress: 10,
            timestamp: Utc::now().to_rfc3339(),
        });

        match manager {
            "apt" => {
                let _ = db.append_log(job_id, "$ sudo -n apt-get update");
                let mut update_cmd = Command::new("sudo");
                update_cmd.args(["-n", "apt-get", "update"]);
                let _ = Self::stream_command_output(job_id, update_cmd, db, tx_log, 30).await;

                let _ = db.append_log(job_id, "$ sudo -n apt-get upgrade -y");
                let mut upgrade_cmd = Command::new("sudo");
                upgrade_cmd.args(["-n", "apt-get", "upgrade", "-y"]);
                Self::stream_command_output(job_id, upgrade_cmd, db, tx_log, 70).await?;
            }
            "dnf" => {
                let _ = db.append_log(job_id, "$ sudo -n dnf upgrade -y");
                let mut cmd = Command::new("sudo");
                cmd.args(["-n", "dnf", "upgrade", "-y"]);
                Self::stream_command_output(job_id, cmd, db, tx_log, 50).await?;
            }
            "pacman" => {
                let _ = db.append_log(job_id, "$ sudo -n pacman -Syu --noconfirm");
                let mut cmd = Command::new("sudo");
                cmd.args(["-n", "pacman", "-Syu", "--noconfirm"]);
                Self::stream_command_output(job_id, cmd, db, tx_log, 50).await?;
            }
            _ => return Err(format!("Unsupported package manager: {}", manager)),
        }

        crate::api::pkgmgr::invalidate_upgradable_cache();
        Ok("System packages upgraded successfully".to_string())
    }

    async fn execute_db_backup(
        job_id: &str,
        engine: &str,
        db_name: &str,
        container_id: Option<&str>,
        target_path: &PathBuf,
        db: &Arc<JobDatabase>,
        tx_log: &broadcast::Sender<JobLogEvent>,
    ) -> Result<String, String> {
        let filename = target_path
            .file_name()
            .map(|f| f.to_string_lossy().to_string())
            .unwrap_or_else(|| "backup.sql".to_string());

        let _ = db.append_log(
            job_id,
            &format!(
                "--- Starting backup for {} (engine: {}) -> {} ---",
                db_name, engine, filename
            ),
        );

        let file = std::fs::File::create(target_path)
            .map_err(|e| format!("Failed to create backup target file: {}", e))?;

        let stdio_file = std::process::Stdio::from(file);

        let mut cmd = if engine == "mysql" {
            if let Some(cid) = container_id {
                let mut c = Command::new("docker");
                c.args(["exec", cid, "mysqldump", "-uroot", db_name]);
                c
            } else {
                let mut c = Command::new("mysqldump");
                c.args(["-uroot", db_name]);
                c
            }
        } else if engine == "postgres" {
            if let Some(cid) = container_id {
                let mut c = Command::new("docker");
                c.args(["exec", cid, "pg_dump", "-U", "postgres", db_name]);
                c
            } else {
                let mut c = Command::new("sudo");
                c.args(["-n", "-u", "postgres", "pg_dump", db_name]);
                c
            }
        } else {
            return Err(format!("Unsupported engine: {}", engine));
        };

        cmd.stdout(stdio_file);
        cmd.stderr(Stdio::piped());

        let mut child = cmd
            .spawn()
            .map_err(|e| format!("Failed to spawn backup process: {}", e))?;

        if let Some(stderr) = child.stderr.take() {
            let db_c = db.clone();
            let tx_c = tx_log.clone();
            let jid = job_id.to_string();
            tokio::spawn(async move {
                let reader = BufReader::new(stderr);
                let mut lines = reader.lines();
                while let Ok(Some(line)) = lines.next_line().await {
                    let _ = db_c.append_log(&jid, &line);
                    let _ = tx_c.send(JobLogEvent {
                        job_id: jid.clone(),
                        event_type: "log".to_string(),
                        line,
                        status: "running".to_string(),
                        progress: 50,
                        timestamp: Utc::now().to_rfc3339(),
                    });
                }
            });
        }

        let status = child
            .wait()
            .await
            .map_err(|e| format!("Backup wait failed: {}", e))?;

        if status.success() {
            let _ = db.append_log(
                job_id,
                &format!("Backup saved successfully to {}", filename),
            );
            Ok(format!("Backup created: {}", filename))
        } else {
            let _ = std::fs::remove_file(target_path);
            Err(format!(
                "Backup tool exited with error code: {:?}",
                status.code()
            ))
        }
    }

    async fn execute_app_install(
        job_id: &str,
        app_id: &str,
        app_name: &str,
        app_dir: &PathBuf,
        db: &Arc<JobDatabase>,
        tx_log: &broadcast::Sender<JobLogEvent>,
    ) -> Result<String, String> {
        let _ = db.append_log(
            job_id,
            &format!(
                "--- Deploying {} ({}) with Docker Compose ---",
                app_name, app_id
            ),
        );

        let mut cmd1 = Command::new("sudo");
        cmd1.args(["-n", "docker-compose", "up", "-d"])
            .current_dir(app_dir);

        let res1 = Self::stream_command_output(job_id, cmd1, db, tx_log, 50).await;
        if res1.is_err() {
            let _ = db.append_log(
                job_id,
                "docker-compose failed, falling back to 'docker compose'...",
            );
            let mut cmd2 = Command::new("sudo");
            cmd2.args(["-n", "docker", "compose", "up", "-d"])
                .current_dir(app_dir);
            Self::stream_command_output(job_id, cmd2, db, tx_log, 75).await?;
        }

        Ok(format!("App {} deployed and containers started", app_name))
    }

    async fn execute_command_job(
        job_id: &str,
        cmd_str: &str,
        args: &[String],
        output_file: Option<&PathBuf>,
        description: &str,
        db: &Arc<JobDatabase>,
        tx_log: &broadcast::Sender<JobLogEvent>,
    ) -> Result<String, String> {
        let _ = db.append_log(job_id, &format!("--- Starting: {} ---", description));
        let _ = tx_log.send(JobLogEvent {
            job_id: job_id.to_string(),
            event_type: "log".to_string(),
            line: format!("--- Starting: {} ---", description),
            status: "running".to_string(),
            progress: 10,
            timestamp: Utc::now().to_rfc3339(),
        });

        let mut cmd = Command::new(cmd_str);
        cmd.args(args);

        if let Some(out_path) = output_file {
            let file = std::fs::File::create(out_path)
                .map_err(|e| format!("Failed to create output file: {}", e))?;
            cmd.stdout(std::process::Stdio::from(file));
            cmd.stderr(Stdio::piped());

            let mut child = cmd
                .spawn()
                .map_err(|e| format!("Failed to spawn command: {}", e))?;
            if let Some(stderr) = child.stderr.take() {
                let db_c = db.clone();
                let tx_c = tx_log.clone();
                let jid = job_id.to_string();
                tokio::spawn(async move {
                    let reader = BufReader::new(stderr);
                    let mut lines = reader.lines();
                    while let Ok(Some(line)) = lines.next_line().await {
                        let _ = db_c.append_log(&jid, &line);
                        let _ = tx_c.send(JobLogEvent {
                            job_id: jid.clone(),
                            event_type: "log".to_string(),
                            line,
                            status: "running".to_string(),
                            progress: 50,
                            timestamp: Utc::now().to_rfc3339(),
                        });
                    }
                });
            }

            let status = child
                .wait()
                .await
                .map_err(|e| format!("Command wait failed: {}", e))?;
            if status.success() {
                let _ = db.append_log(
                    job_id,
                    &format!("Task {} completed successfully", description),
                );
                Ok(format!("Task {} completed successfully", description))
            } else {
                let _ = std::fs::remove_file(out_path);
                Err(format!("Command exited with status: {:?}", status.code()))
            }
        } else {
            Self::stream_command_output(job_id, cmd, db, tx_log, 50).await?;
            let _ = db.append_log(
                job_id,
                &format!("Task {} completed successfully", description),
            );
            Ok(format!("Task {} completed successfully", description))
        }
    }

    async fn execute_acme_issue(
        job_id: &str,
        domain: &str,
        email: &str,
        challenge_store: &crate::ssl::AcmeChallengeStore,
        ssl_db: &Arc<crate::ssl::SslDatabase>,
        db: &Arc<JobDatabase>,
        tx_log: &broadcast::Sender<JobLogEvent>,
    ) -> Result<String, String> {
        let jid = job_id.to_string();
        let jdb = db.clone();
        let tx = tx_log.clone();

        let progress_cb = move |progress: i32, msg: &str| {
            let _ = jdb.update_status(&jid, "running", progress, None);
            let _ = jdb.append_log(&jid, msg);
            let _ = tx.send(JobLogEvent {
                job_id: jid.clone(),
                event_type: "log".to_string(),
                line: msg.to_string(),
                status: "running".to_string(),
                progress,
                timestamp: Utc::now().to_rfc3339(),
            });
        };

        let res = crate::ssl::acme::issue_letsencrypt_certificate(
            domain,
            email,
            challenge_store,
            progress_cb,
        )
        .await?;

        let cert_str = res.cert_path.to_string_lossy().to_string();
        let key_str = res.key_path.to_string_lossy().to_string();

        ssl_db
            .update_certificate(&crate::ssl::CertMetaUpdate {
                mode: "lets_encrypt",
                domain: Some(&res.domain),
                email: Some(email),
                cert_path: &cert_str,
                key_path: &key_str,
                issuer: "Let's Encrypt",
                expires_at: Some(res.expires_at),
            })
            .map_err(|e| format!("Failed to update SSL database: {}", e))?;

        let _ = ssl_db.set_enabled(true, true);

        Ok(format!(
            "Let's Encrypt certificate for {} successfully issued and activated",
            domain
        ))
    }

    pub async fn enqueue_job(
        &self,
        job_type: &str,
        payload: JobTaskPayload,
    ) -> Result<String, String> {
        let id = Uuid::new_v4().to_string();
        self.db
            .create_job(&id, job_type)
            .map_err(|e| format!("Failed to create job record: {}", e))?;

        let _ = Self::get_or_create_broadcaster(&self.broadcasters, &id);

        let task = JobTask {
            id: id.clone(),
            job_type: job_type.to_string(),
            payload,
        };

        self.task_tx
            .send(task)
            .await
            .map_err(|e| format!("Failed to queue task: {}", e))?;

        Ok(id)
    }

    pub fn get_job(&self, id: &str) -> Result<Option<Job>, rusqlite::Error> {
        self.db.get_job(id)
    }

    pub fn list_jobs(&self, limit: usize) -> Result<Vec<JobSummary>, rusqlite::Error> {
        self.db.list_jobs(limit)
    }

    pub fn subscribe_logs(&self, id: &str) -> broadcast::Receiver<JobLogEvent> {
        let tx = Self::get_or_create_broadcaster(&self.broadcasters, id);
        tx.subscribe()
    }
}

pub fn get_wadm_db_path() -> PathBuf {
    if let Ok(data_dir) = std::env::var("WADM_DATA_DIR") {
        let p = PathBuf::from(data_dir);
        let _ = std::fs::create_dir_all(&p);
        return p.join("wadm.db");
    }
    let system_dir = std::path::Path::new("/var/lib/wadm");
    if system_dir.exists() && std::fs::create_dir_all(system_dir).is_ok() {
        return system_dir.join("wadm.db");
    }
    let local_dir = std::env::current_dir().unwrap_or_default().join("data");
    let _ = std::fs::create_dir_all(&local_dir);
    local_dir.join("wadm.db")
}

// ============================================================================
// REST API Handlers
// ============================================================================

pub async fn list_jobs(manager: web::Data<Arc<JobManager>>) -> impl Responder {
    match manager.list_jobs(50) {
        Ok(jobs) => HttpResponse::Ok().json(jobs),
        Err(e) => {
            log::error!("Failed to list jobs: {}", e);
            HttpResponse::InternalServerError().json("Failed to load jobs list")
        }
    }
}

pub async fn get_job(
    path: web::Path<String>,
    manager: web::Data<Arc<JobManager>>,
) -> impl Responder {
    let job_id = path.into_inner();
    match manager.get_job(&job_id) {
        Ok(Some(job)) => HttpResponse::Ok().json(job),
        Ok(None) => HttpResponse::NotFound().json("Job not found"),
        Err(e) => {
            log::error!("Failed to query job {}: {}", job_id, e);
            HttpResponse::InternalServerError().json("Failed to query job")
        }
    }
}

pub async fn stream_job_logs(
    path: web::Path<String>,
    manager: web::Data<Arc<JobManager>>,
) -> impl Responder {
    let job_id = path.into_inner();

    let job = match manager.get_job(&job_id) {
        Ok(Some(j)) => j,
        Ok(None) => return HttpResponse::NotFound().body("Job not found"),
        Err(e) => {
            log::error!("Error reading job for stream: {}", e);
            return HttpResponse::InternalServerError().body("Database error");
        }
    };

    let rx = manager.subscribe_logs(&job_id);
    let initial_status = job.status.clone();
    let initial_logs: Vec<String> = job.logs.lines().map(|s| s.to_string()).collect();

    let stream = futures_util::stream::unfold(
        (rx, initial_logs, initial_status, job_id, false),
        |(mut rx, mut pending_lines, status, jid, finished)| async move {
            if finished {
                return None;
            }

            // First, drain any historical lines from SQLite
            if !pending_lines.is_empty() {
                let line = pending_lines.remove(0);
                let evt = JobLogEvent {
                    job_id: jid.clone(),
                    event_type: "log".to_string(),
                    line,
                    status: status.clone(),
                    progress: if status == "completed" { 100 } else { 0 },
                    timestamp: Utc::now().to_rfc3339(),
                };
                let json = serde_json::to_string(&evt).unwrap_or_default();
                let sse_chunk = format!("data: {}\n\n", json);
                return Some((
                    Ok::<actix_web::web::Bytes, actix_web::Error>(actix_web::web::Bytes::from(
                        sse_chunk,
                    )),
                    (rx, pending_lines, status, jid, false),
                ));
            }

            // If the job was already completed or failed before connection, send termination event and finish
            if status == "completed" || status == "failed" {
                let evt = JobLogEvent {
                    job_id: jid.clone(),
                    event_type: "complete".to_string(),
                    line: format!("Job ended with status: {}", status),
                    status: status.clone(),
                    progress: if status == "completed" { 100 } else { -1 },
                    timestamp: Utc::now().to_rfc3339(),
                };
                let json = serde_json::to_string(&evt).unwrap_or_default();
                let sse_chunk = format!("data: {}\n\n", json);
                return Some((
                    Ok::<actix_web::web::Bytes, actix_web::Error>(actix_web::web::Bytes::from(
                        sse_chunk,
                    )),
                    (rx, pending_lines, status, jid, true),
                ));
            }

            // Live event streaming loop
            loop {
                match rx.recv().await {
                    Ok(event) => {
                        let is_end = event.event_type == "complete" || event.event_type == "error";
                        let json = serde_json::to_string(&event).unwrap_or_default();
                        let sse_chunk = format!("data: {}\n\n", json);
                        return Some((
                            Ok::<actix_web::web::Bytes, actix_web::Error>(
                                actix_web::web::Bytes::from(sse_chunk),
                            ),
                            (rx, pending_lines, event.status, jid, is_end),
                        ));
                    }
                    Err(broadcast::error::RecvError::Lagged(missed)) => {
                        log::debug!("Client lagged by {} job log events, catching up...", missed);
                        continue;
                    }
                    Err(broadcast::error::RecvError::Closed) => {
                        log::info!("Job log channel closed for {}", jid);
                        return None;
                    }
                }
            }
        },
    );

    HttpResponse::Ok()
        .insert_header((actix_web::http::header::CONTENT_TYPE, "text/event-stream"))
        .insert_header((
            actix_web::http::header::CACHE_CONTROL,
            "no-cache, no-transform",
        ))
        .insert_header((actix_web::http::header::CONNECTION, "keep-alive"))
        .insert_header(("X-Accel-Buffering", "no"))
        .streaming(stream)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_job_database_lifecycle() {
        let db = JobDatabase::new_in_memory().expect("In-memory SQLite failed");
        let job = db
            .create_job("test-job-123", "package_upgrade")
            .expect("Create failed");
        assert_eq!(job.id, "test-job-123");
        assert_eq!(job.status, "pending");

        db.append_log("test-job-123", "Updating package lists...")
            .expect("Log failed");
        db.append_log("test-job-123", "Upgraded 12 packages.")
            .expect("Log failed");

        db.update_status("test-job-123", "completed", 100, None)
            .expect("Update status failed");

        let fetched = db
            .get_job("test-job-123")
            .expect("Get failed")
            .expect("Job missing");
        assert_eq!(fetched.status, "completed");
        assert_eq!(fetched.progress, 100);
        assert!(fetched.logs.contains("Updating package lists..."));
        assert!(fetched.logs.contains("Upgraded 12 packages."));

        let list = db.list_jobs(10).expect("List failed");
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].id, "test-job-123");
    }

    #[tokio::test]
    async fn test_job_manager_broadcast() {
        let db = Arc::new(JobDatabase::new_in_memory().expect("In-memory SQLite failed"));
        let manager = JobManager::new(db);

        let mut rx = manager.subscribe_logs("test-broadcast-1");
        let tx = JobManager::get_or_create_broadcaster(&manager.broadcasters, "test-broadcast-1");

        let event = JobLogEvent {
            job_id: "test-broadcast-1".to_string(),
            event_type: "log".to_string(),
            line: "Hello worker".to_string(),
            status: "running".to_string(),
            progress: 10,
            timestamp: Utc::now().to_rfc3339(),
        };

        tx.send(event).expect("Send failed");
        let received = rx.recv().await.expect("Receive failed");
        assert_eq!(received.line, "Hello worker");
        assert_eq!(received.status, "running");
    }
}
