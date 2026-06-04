import React from 'react';
import { Document } from '../types';

interface DocumentViewerProps {
  document: Document | null;
  onClose: () => void;
}

export const DocumentViewer: React.FC<DocumentViewerProps> = ({ document, onClose }) => {
  if (!document) {
    return null;
  }

  const currentVersion = document.versions.find(v => v.id === document.currentVersionId);

  return (
    <div className="document-viewer-overlay" onClick={onClose}>
      <div className="document-viewer" onClick={(e) => e.stopPropagation()}>
        <div className="document-header">
          <h2>{document.title}</h2>
          <div className="document-meta">
            <span>Author: {document.authorName}</span>
            <span>Created: {new Date(document.createdAt).toLocaleDateString()}</span>
            <span>Updated: {new Date(document.updatedAt).toLocaleDateString()}</span>
            <span>Version: {document.versions.length}</span>
          </div>
          <button className="close-btn" onClick={onClose}>×</button>
        </div>

        <div className="document-content">
          {currentVersion ? (
            <div className="version-content">
              <div className="version-info">
                <strong>Current Version</strong>
                {currentVersion.changeSummary && (
                  <p className="change-summary">Changes: {currentVersion.changeSummary}</p>
                )}
                <small>Created: {new Date(currentVersion.createdAt).toLocaleString()}</small>
              </div>
              <div className="markdown-content">
                <pre>{currentVersion.content}</pre>
              </div>
            </div>
          ) : (
            <p>No content available</p>
          )}
        </div>

        {document.versions.length > 1 && (
          <div className="version-history">
            <h3>Version History</h3>
            <ul className="version-list">
              {document.versions.map((version, index) => (
                <li
                  key={version.id}
                  className={`version-item ${version.id === document.currentVersionId ? 'current' : ''}`}
                >
                  <span className="version-number">v{index + 1}</span>
                  <span className="version-date">
                    {new Date(version.createdAt).toLocaleString()}
                  </span>
                  {version.changeSummary && (
                    <span className="version-summary">{version.changeSummary}</span>
                  )}
                </li>
              ))}
            </ul>
          </div>
        )}

        <div className="document-actions">
          <button className="action-btn" onClick={() => window.open(`file://${document.versions[0]?.content}`, '_blank')}>
            Open in External Editor
          </button>
          <button className="action-btn" onClick={() => navigator.clipboard.writeText(currentVersion?.content || '')}>
            Copy Content
          </button>
        </div>
      </div>
    </div>
  );
};
