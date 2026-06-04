import React, { useState } from 'react';
import { ChatMessage } from '../types';
import { VoteTooltip } from './VoteTooltip';

interface ChatMessageProps {
  message: ChatMessage;
  onVote: (messageId: string, voteType: 'approve' | 'disapprove') => void;
  onActionRequest: (action: { type: 'request_counter_proposal' | 'state_reason'; targetAgentId: string; messageId: string }) => void;
  onDocumentClick: (documentId: string) => void;
}

const WORD_LIMIT = 200;

export const ChatMessageComponent: React.FC<ChatMessageProps> = ({
  message,
  onVote,
  onActionRequest,
  onDocumentClick,
}) => {
  const [showFullContent, setShowFullContent] = useState(false);
  const [showTooltip, setShowTooltip] = useState(false);

  const wordCount = message.content.split(/\s+/).length;
  const isTruncated = wordCount > WORD_LIMIT;
  const displayedContent = showFullContent || !isTruncated
    ? message.content
    : message.content.split(/\s+/).slice(0, WORD_LIMIT).join(' ') + '...';

  const handleToggleContent = () => {
    if (isTruncated) {
      setShowFullContent(!showFullContent);
    }
  };

  const totalApprove = message.votes.approve.length;
  const totalDisapprove = message.votes.disapprove.length;

  const renderDocumentLinks = () => {
    if (!message.documentLinks || message.documentLinks.length === 0) return null;

    return (
      <div className="document-links">
        <span className="document-label">📄 Attachments:</span>
        {message.documentLinks.map((link) => (
          <button
            key={link.documentId}
            className="document-link"
            onClick={() => onDocumentClick(link.documentId)}
          >
            {link.title}
          </button>
        ))}
      </div>
    );
  };

  const handleTooltipAction = (action: { type: 'request_counter_proposal' | 'state_reason'; targetAgentId: string; messageId: string }) => {
    onActionRequest(action);
    setShowTooltip(false);
  };

  return (
    <div className={`chat-message ${message.priority}`}>
      <div className="message-header">
        <span className="sender-name">{message.senderName}</span>
        <span className="message-time">
          {new Date(message.timestamp).toLocaleTimeString()}
        </span>
      </div>

      <div className="message-content">
        <p>{displayedContent}</p>
        {isTruncated && (
          <button
            className="toggle-content-btn"
            onClick={handleToggleContent}
          >
            {showFullContent ? 'Show less' : 'Show more'}
          </button>
        )}
        {renderDocumentLinks()}
      </div>

      <div className="message-footer">
        <div className="vote-controls">
          <button
            className="vote-btn approve"
            onClick={() => onVote(message.id, 'approve')}
            title="Approve this message"
          >
            👍
          </button>
          <button
            className="vote-btn disapprove"
            onClick={() => onVote(message.id, 'disapprove')}
            title="Disapprove this message"
          >
            👎
          </button>
        </div>

        <div
          className="vote-count"
          onMouseEnter={() => setShowTooltip(true)}
          onMouseLeave={() => setShowTooltip(false)}
        >
          <span className="approve-count">👍 {totalApprove}</span>
          <span className="separator">|</span>
          <span className="disapprove-count">👎 {totalDisapprove}</span>

          {showTooltip && (
            <div className="tooltip-container">
              <VoteTooltip
                messageId={message.id}
                approveVotes={message.votes.approve}
                disapproveVotes={message.votes.disapprove}
                onActionRequest={handleTooltipAction}
              />
            </div>
          )}
        </div>
      </div>
    </div>
  );
};
