import React, { useState } from 'react';
import { PrivateMessage, PMEntryType, PermissionRequest, PermissionStatus } from '../types';

interface PMTabProps {
  pm: PrivateMessage;
  isActive: boolean;
  onActivate: () => void;
  onClose: () => void;
  onGrantPermission: (requestId: string) => void;
  onDenyPermission: (requestId: string) => void;
  onSendMessage: (pmId: string, content: string) => void;
  onSubmitWork: (pmId: string) => void;
}

export const PMTab: React.FC<PMTabProps> = ({
  pm,
  isActive,
  onActivate,
  onClose,
  onGrantPermission,
  onDenyPermission,
  onSendMessage,
  onSubmitWork,
}) => {
  const [inputValue, setInputValue] = useState('');

  const handleSend = () => {
    if (inputValue.trim()) {
      // Check for /submit command
      if (inputValue.trim() === '/submit') {
        onSubmitWork(pm.id);
      } else {
        onSendMessage(pm.id, inputValue);
      }
      setInputValue('');
    }
  };

  const handleKeyPress = (e: React.KeyboardEvent) => {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      handleSend();
    }
  };

  const copyWorkLog = async () => {
    // Find the latest work log block
    const workLogEntries = pm.messages
      .filter(m => m.type === PMEntryType.WorkLog && m.workLogBlock)
      .flatMap(m => m.workLogBlock!.entries);

    const workLogText = workLogEntries
      .map(entry => `[${new Date(entry.timestamp).toLocaleTimeString()}] ${entry.content}`)
      .join('\n');

    try {
      await navigator.clipboard.writeText(workLogText);
      alert('Work log copied to clipboard!');
    } catch (err) {
      console.error('Failed to copy work log:', err);
    }
  };

  const getOtherParticipant = () => {
    // Assuming human user is always one participant
    const otherId = pm.participants.find(p => p !== 'human-user');
    return otherId || 'Unknown';
  };

  const pendingRequests = pm.permissionRequests.filter(
    r => r.status === PermissionStatus.Pending
  );

  return (
    <div className={`pm-tab ${isActive ? 'active' : 'inactive'}`} onClick={onActivate}>
      <div className="pm-tab-header">
        <span className="pm-tab-title">PM: {getOtherParticipant()}</span>
        {isActive && (
          <button className="close-pm-btn" onClick={(e) => { e.stopPropagation(); onClose(); }}>
            ×
          </button>
        )}
      </div>

      {isActive && (
        <div className="pm-content">
          <div className="pm-messages">
            {pm.messages.map((entry) => (
              <div key={entry.id} className={`pm-entry ${entry.type}`}>
                <span className="pm-entry-time">
                  {new Date(entry.timestamp).toLocaleTimeString()}
                </span>
                <div className="pm-entry-content">
                  {entry.type === PMEntryType.WorkLog && entry.workLogBlock ? (
                    <div className="work-log-block">
                      <div className="work-log-header">
                        <span>📝 Work Log</span>
                        <button onClick={copyWorkLog} className="copy-worklog-btn">
                          📋 Copy All
                        </button>
                      </div>
                      <div className="work-log-entries">
                        {entry.workLogBlock.entries.map((logEntry, idx) => (
                          <div key={idx} className={`work-log-entry ${logEntry.type}`}>
                            <span className="entry-type">{logEntry.type}:</span>
                            <span className="entry-content">{logEntry.content}</span>
                          </div>
                        ))}
                      </div>
                    </div>
                  ) : (
                    <p>{entry.content}</p>
                  )}
                </div>
              </div>
            ))}

            {/* Pending Permission Requests */}
            {pendingRequests.length > 0 && (
              <div className="permission-requests">
                <h4>Pending Permission Requests:</h4>
                {pendingRequests.map((request) => (
                  <div key={request.id} className="permission-request">
                    <div className="request-info">
                      <strong>{request.agentName}</strong> requests permission to:
                      <p>{request.action}</p>
                      <small>Scope: {request.scope.type}{request.scope.path ? ` - ${request.scope.path}` : ''}</small>
                    </div>
                    <div className="request-actions">
                      <button
                        className="grant-btn"
                        onClick={() => onGrantPermission(request.id)}
                      >
                        ✓ Grant
                      </button>
                      <button
                        className="deny-btn"
                        onClick={() => onDenyPermission(request.id)}
                      >
                        ✗ Deny
                      </button>
                    </div>
                  </div>
                ))}
              </div>
            )}
          </div>

          <div className="pm-input-area">
            <textarea
              value={inputValue}
              onChange={(e) => setInputValue(e.target.value)}
              onKeyPress={handleKeyPress}
              placeholder="Type a message... (use /submit to submit work)"
              rows={3}
            />
            <button onClick={handleSend} className="send-btn">
              Send
            </button>
          </div>
        </div>
      )}
    </div>
  );
};
