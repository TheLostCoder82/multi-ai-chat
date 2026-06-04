import React from 'react';
import { Vote } from '../types';

interface VoteTooltipProps {
  messageId: string;
  approveVotes: Vote[];
  disapproveVotes: Vote[];
  onActionRequest: (action: { type: 'request_counter_proposal' | 'state_reason'; targetAgentId: string; messageId: string }) => void;
}

export const VoteTooltip: React.FC<VoteTooltipProps> = ({
  messageId,
  approveVotes,
  disapproveVotes,
  onActionRequest,
}) => {
  return (
    <div className="vote-tooltip">
      <div className="vote-section">
        <h4 className="vote-section-title">👍 Approved ({approveVotes.length})</h4>
        {approveVotes.length > 0 ? (
          <ul className="voter-list">
            {approveVotes.map((vote) => (
              <li key={vote.agentId} className="voter-item">
                <span className="voter-name">{vote.agentName}</span>
                <div className="voter-actions">
                  <button
                    className="action-btn"
                    onClick={() =>
                      onActionRequest({
                        type: 'request_counter_proposal',
                        targetAgentId: vote.agentId,
                        messageId,
                      })
                    }
                    title="Request counter-proposal from this agent"
                  >
                    Counter
                  </button>
                  <button
                    className="action-btn"
                    onClick={() =>
                      onActionRequest({
                        type: 'state_reason',
                        targetAgentId: vote.agentId,
                        messageId,
                      })
                    }
                    title="Ask for voting reason"
                  >
                    Why?
                  </button>
                </div>
              </li>
            ))}
          </ul>
        ) : (
          <p className="no-voters">No approvals yet</p>
        )}
      </div>

      <div className="vote-section">
        <h4 className="vote-section-title">👎 Disapproved ({disapproveVotes.length})</h4>
        {disapproveVotes.length > 0 ? (
          <ul className="voter-list">
            {disapproveVotes.map((vote) => (
              <li key={vote.agentId} className="voter-item">
                <span className="voter-name">{vote.agentName}</span>
                <div className="voter-actions">
                  <button
                    className="action-btn"
                    onClick={() =>
                      onActionRequest({
                        type: 'request_counter_proposal',
                        targetAgentId: vote.agentId,
                        messageId,
                      })
                    }
                    title="Request counter-proposal from this agent"
                  >
                    Counter
                  </button>
                  <button
                    className="action-btn"
                    onClick={() =>
                      onActionRequest({
                        type: 'state_reason',
                        targetAgentId: vote.agentId,
                        messageId,
                      })
                    }
                    title="Ask for voting reason"
                  >
                    Why?
                  </button>
                </div>
              </li>
            ))}
          </ul>
        ) : (
          <p className="no-voters">No disapprovals yet</p>
        )}
      </div>
    </div>
  );
};
