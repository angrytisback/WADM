import React, { useState, useEffect, useRef } from 'react';
import { 
  FaFolder, FaFile, FaChevronRight, FaUpload, FaDownload, 
  FaTrash, FaEdit, FaFolderPlus, FaArrowUp, FaSave, FaTimes, FaSpinner,
  FaFileMedical
} from 'react-icons/fa';
import { useToast } from '../context/ToastContext';

interface FileInfo {
  name: string;
  path: string;
  is_dir: boolean;
  size: number;
  permissions: string;
  modified_at: string;
}

export const FileExplorer: React.FC = () => {
  const [currentPath, setCurrentPath] = useState('/');
  const [files, setFiles] = useState<FileInfo[]>([]);
  const [loading, setLoading] = useState(true);
  const [editingFile, setEditingFile] = useState<string | null>(null);
  const [fileContent, setFileContent] = useState('');
  const fileInputRef = useRef<HTMLInputElement>(null);
  const { addToast } = useToast();

  const fetchFiles = async (path: string) => {
    setLoading(true);
    try {
      const res = await fetch(`/api/files/list?path=${encodeURIComponent(path)}`);
      if (!res.ok) throw new Error(await res.text());
      const data = await res.json();
      setFiles(data);
      setCurrentPath(path);
    } catch {
      addToast('Failed to read directory', 'error');
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    fetchFiles(currentPath);
  }, []);

  const navigateTo = (path: string) => {
    fetchFiles(path);
  };

  const navigateUp = () => {
    if (currentPath === '/') return;
    const parts = currentPath.split('/').filter(Boolean);
    parts.pop();
    const newPath = '/' + parts.join('/');
    fetchFiles(newPath);
  };

  const handleCreate = async (isDir: boolean) => {
    const name = window.prompt(`Enter ${isDir ? 'directory' : 'file'} name:`);
    if (!name) return;
    
    const targetPath = currentPath.endsWith('/') 
      ? `${currentPath}${name}` 
      : `${currentPath}/${name}`;
      
    try {
      const res = await fetch('/api/files/create', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ path: targetPath, is_dir: isDir })
      });
      if (!res.ok) throw new Error(await res.text());
      addToast(`${isDir ? 'Directory' : 'File'} created successfully`, 'success');
      fetchFiles(currentPath);
    } catch {
      addToast(`Failed to create ${isDir ? 'directory' : 'file'}`, 'error');
    }
  };

  const handleDelete = async (file: FileInfo) => {
    if (!window.confirm(`Are you sure you want to delete ${file.name}?`)) return;
    try {
      const res = await fetch(`/api/files/delete?path=${encodeURIComponent(file.path)}`, {
        method: 'DELETE'
      });
      if (!res.ok) throw new Error(await res.text());
      addToast('Deleted successfully', 'success');
      fetchFiles(currentPath);
    } catch {
      addToast('Failed to delete item', 'error');
    }
  };

  const handleEdit = async (file: FileInfo) => {
    if (file.is_dir) return;
    if (file.size > 5 * 1024 * 1024) {
      addToast('File too large to edit in browser (max 5MB)', 'error');
      return;
    }
    try {
      const res = await fetch(`/api/files/read?path=${encodeURIComponent(file.path)}`);
      if (!res.ok) throw new Error(await res.text());
      const data = await res.json();
      setFileContent(data);
      setEditingFile(file.path);
    } catch {
      addToast('Failed to read file or file is binary', 'error');
    }
  };

  const saveFile = async () => {
    if (!editingFile) return;
    try {
      const res = await fetch('/api/files/write', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ path: editingFile, content: fileContent })
      });
      if (!res.ok) throw new Error(await res.text());
      addToast('File saved successfully', 'success');
      setEditingFile(null);
    } catch {
      addToast('Failed to save file', 'error');
    }
  };

  const handleUpload = async (event: React.ChangeEvent<HTMLInputElement>) => {
    const uploadedFiles = event.target.files;
    if (!uploadedFiles || uploadedFiles.length === 0) return;

    const formData = new FormData();
    for (let i = 0; i < uploadedFiles.length; i++) {
      formData.append('file', uploadedFiles[i]);
    }

    try {
      addToast('Uploading file(s)...', 'info');
      const res = await fetch(`/api/files/upload?path=${encodeURIComponent(currentPath)}`, {
        method: 'POST',
        body: formData
      });
      if (!res.ok) throw new Error(await res.text());
      addToast('Upload complete', 'success');
      fetchFiles(currentPath);
    } catch {
      addToast('Upload failed', 'error');
    }
  };

  const getBreadcrumbs = () => {
    const parts = currentPath.split('/').filter(Boolean);
    return (
      <div style={{ display: 'flex', alignItems: 'center', gap: '0.4rem', fontSize: '0.9rem', color: 'var(--text-secondary)', overflowX: 'auto' }}>
        <button onClick={() => navigateTo('/')} style={{ background: 'none', border: 'none', color: 'var(--accent-color)', cursor: 'pointer', fontWeight: 600 }}>
          /
        </button>
        {parts.map((part, index) => {
          const path = '/' + parts.slice(0, index + 1).join('/');
          return (
            <React.Fragment key={path}>
              <FaChevronRight style={{ fontSize: '0.7rem', opacity: 0.5 }} />
              <button 
                onClick={() => navigateTo(path)}
                style={{ background: 'none', border: 'none', color: 'var(--text-primary)', cursor: 'pointer', fontWeight: 500 }}
              >
                {part}
              </button>
            </React.Fragment>
          );
        })}
      </div>
    );
  };

  if (editingFile) {
    return (
      <div className="glass-panel" style={{ padding: '1.5rem', height: 'calc(100vh - 10rem)', display: 'flex', flexDirection: 'column' }}>
        <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '1rem' }}>
          <h3 style={{ margin: 0, fontSize: '1.1rem', fontWeight: 600, color: 'var(--text-primary)' }}>
            Editing: {editingFile}
          </h3>
          <div style={{ display: 'flex', gap: '0.5rem' }}>
            <button
              onClick={saveFile}
              className="btn primary"
              style={{ display: 'flex', alignItems: 'center', gap: '0.4rem', padding: '0.4rem 0.8rem' }}
            >
              <FaSave />
              <span>Save</span>
            </button>
            <button
              onClick={() => setEditingFile(null)}
              className="btn"
              style={{ display: 'flex', alignItems: 'center', gap: '0.4rem', padding: '0.4rem 0.8rem', background: 'rgba(255,255,255,0.1)' }}
            >
              <FaTimes />
              <span>Cancel</span>
            </button>
          </div>
        </div>
        <textarea
          value={fileContent}
          onChange={(e) => setFileContent(e.target.value)}
          style={{
            flex: 1,
            width: '100%',
            background: 'rgba(15, 23, 42, 0.6)',
            border: '1px solid var(--glass-border)',
            borderRadius: '8px',
            padding: '1rem',
            color: '#e2e8f0',
            fontFamily: 'monospace',
            fontSize: '0.9rem',
            resize: 'none',
            outline: 'none'
          }}
        />
      </div>
    );
  }

  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: '1.5rem', padding: '0 1rem' }} className="fade-in">
      <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', flexWrap: 'wrap', gap: '1rem' }}>
        <div>
          <h2 style={{ fontSize: '1.75rem', fontWeight: 700, margin: 0, color: 'var(--text-primary)' }}>File Explorer</h2>
          <div style={{ marginTop: '0.5rem' }}>{getBreadcrumbs()}</div>
        </div>
        <div style={{ display: 'flex', gap: '0.5rem' }}>
          <button onClick={() => handleCreate(true)} className="btn" style={{ background: 'rgba(255,255,255,0.05)', display: 'flex', alignItems: 'center', gap: '0.4rem', padding: '0.4rem 0.8rem' }} title="New Directory">
            <FaFolderPlus />
            <span>New Folder</span>
          </button>
          <button onClick={() => handleCreate(false)} className="btn" style={{ background: 'rgba(255,255,255,0.05)', display: 'flex', alignItems: 'center', gap: '0.4rem', padding: '0.4rem 0.8rem' }} title="New File">
            <FaFileMedical />
            <span>New File</span>
          </button>
          <button onClick={() => fileInputRef.current?.click()} className="btn primary" style={{ display: 'flex', alignItems: 'center', gap: '0.4rem', padding: '0.4rem 0.8rem' }} title="Upload File">
            <FaUpload />
            <span>Upload</span>
          </button>
          <input type="file" ref={fileInputRef} style={{ display: 'none' }} multiple onChange={handleUpload} />
        </div>
      </div>

      <div className="glass-panel" style={{ overflow: 'hidden' }}>
        <div style={{ overflowX: 'auto' }}>
          <table style={{ width: '100%', borderCollapse: 'separate', borderSpacing: 0, textAlign: 'left' }}>
            <thead>
              <tr style={{ background: 'rgba(255,255,255,0.02)' }}>
                <th style={{ padding: '1rem 1.2rem', color: 'var(--text-secondary)', fontWeight: 600, borderBottom: '1px solid var(--glass-border)' }}>Name</th>
                <th style={{ padding: '1rem 1.2rem', color: 'var(--text-secondary)', fontWeight: 600, borderBottom: '1px solid var(--glass-border)' }}>Size</th>
                <th style={{ padding: '1rem 1.2rem', color: 'var(--text-secondary)', fontWeight: 600, borderBottom: '1px solid var(--glass-border)' }}>Permissions</th>
                <th style={{ padding: '1rem 1.2rem', color: 'var(--text-secondary)', fontWeight: 600, borderBottom: '1px solid var(--glass-border)' }}>Modified</th>
                <th style={{ padding: '1rem 1.2rem', color: 'var(--text-secondary)', fontWeight: 600, borderBottom: '1px solid var(--glass-border)', textAlign: 'right' }}>Actions</th>
              </tr>
            </thead>
            <tbody>
              {currentPath !== '/' && (
                <tr className="table-row-hover" style={{ cursor: 'pointer' }} onClick={navigateUp}>
                  <td style={{ padding: '0.8rem 1.2rem', display: 'flex', alignItems: 'center', gap: '0.75rem', color: 'var(--accent-color)' }}>
                    <FaArrowUp />
                    <span>..</span>
                  </td>
                  <td colSpan={4} style={{ padding: '0.8rem 1.2rem' }}></td>
                </tr>
              )}
              {files.map((file) => (
                <tr 
                  key={file.path} 
                  className="table-row-hover"
                >
                  <td 
                    style={{ padding: '0.8rem 1.2rem', display: 'flex', alignItems: 'center', gap: '0.75rem', cursor: 'pointer' }}
                    onClick={() => file.is_dir ? navigateTo(file.path) : handleEdit(file)}
                  >
                    {file.is_dir ? (
                      <FaFolder style={{ color: '#38bdf8', fontSize: '1.2rem' }} />
                    ) : (
                      <FaFile style={{ color: '#94a3b8', fontSize: '1.1rem' }} />
                    )}
                    <span style={{ fontWeight: 500, color: file.is_dir ? 'var(--text-primary)' : 'var(--text-secondary)' }}>{file.name}</span>
                  </td>
                  <td style={{ padding: '0.8rem 1.2rem', fontSize: '0.85rem', color: 'var(--text-secondary)' }}>
                    {file.is_dir ? '--' : (file.size / 1024).toFixed(2) + ' KB'}
                  </td>
                  <td style={{ padding: '0.8rem 1.2rem', fontSize: '0.85rem', fontFamily: 'monospace', color: 'var(--text-secondary)' }}>
                    {file.permissions}
                  </td>
                  <td style={{ padding: '0.8rem 1.2rem', fontSize: '0.85rem', color: 'var(--text-secondary)' }}>
                    {file.modified_at}
                  </td>
                  <td style={{ padding: '0.8rem 1.2rem', textAlign: 'right' }}>
                    <div style={{ display: 'flex', justifyContent: 'flex-end', gap: '0.5rem' }}>
                      {!file.is_dir && (
                        <button
                          onClick={() => {
                            window.open(`/api/files/download?path=${encodeURIComponent(file.path)}`, '_blank');
                          }}
                          className="btn-text"
                          title="Download"
                          style={{ padding: '0.3rem 0.5rem', color: 'var(--text-secondary)' }}
                        >
                          <FaDownload />
                        </button>
                      )}
                      {!file.is_dir && (
                        <button
                          onClick={() => handleEdit(file)}
                          className="btn-text"
                          title="Edit"
                          style={{ padding: '0.3rem 0.5rem', color: 'var(--accent-color)' }}
                        >
                          <FaEdit />
                        </button>
                      )}
                      <button
                        onClick={() => handleDelete(file)}
                        className="btn-text danger"
                        title="Delete"
                        style={{ padding: '0.3rem 0.5rem' }}
                      >
                        <FaTrash />
                      </button>
                    </div>
                  </td>
                </tr>
              ))}
              {files.length === 0 && !loading && (
                <tr>
                  <td colSpan={5} style={{ padding: '2rem', textAlign: 'center', color: 'var(--text-secondary)' }}>
                    This directory is empty.
                  </td>
                </tr>
              )}
              {loading && (
                <tr>
                  <td colSpan={5} style={{ padding: '2rem', textAlign: 'center', color: 'var(--text-secondary)' }}>
                    <FaSpinner className="animate-spin" style={{ marginRight: '0.5rem' }} /> Loading files...
                  </td>
                </tr>
              )}
            </tbody>
          </table>
        </div>
      </div>
    </div>
  );
};
