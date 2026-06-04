import React, { useState } from 'react';
import { useSelector, useDispatch } from 'react-redux';
import { RootState, AppDispatch } from './store';
import { ChatMessageComponent } from './components/ChatMessage';
import { PMTab } from './components/PMTab';
import { DocumentViewer } from './components/DocumentViewer';
import { useWebSocket } from './hooks/useWebSocket';
import { McpMethod, Vote, ChatMessage as ChatMessageType, Document as DocumentType } from './types';
import {
  addMessage,
  updateMessageVotes,
  addAgent,
  addDocument,
  addPMEntry,
  addPermissionRequest,
  updatePermissionStatus,
  setActivePMTab,
  setConnected,
} from './store';

const WS_URL = 'ws://localhost:8080/ws';

const App: React.FC = () => {
  const dispatch = useDispatch<AppDispatch>();
  const state = useSelector((state: RootState) => state.app);
  
  const [selectedDocument, setSelectedDocument] = useState<DocumentType | null>(null);
  const [messageInput, setMessageInput] = useState('');

  // WebSocket connection
  const { isConnected, sendRequest } = useWebSocket({
    url: WS_URL,
    onOpen: () => {
      dispatch(setConnected(true));
      sendRequest(McpMethod.Initialize, {
        clientName: 'Human User',
        clientId: state.humanUserId,
      }).catch(console.error);
    },
    onClose: () => dispatch(setConnected(false)),
    onMessage: handleWebSocketMessage,
  });

  function handleWebSocketMessage(message: any) {
    console.log('Received message:', message);
    
    if (message.type === 'notification') {
      switch (message.method) {
        case 'newMessage':
          dispatch(addMessage(message.params as ChatMessageType));
          break;
        case 'agentConnected':
          dispatch(addAgent(message.params));
          break;
        case 'permissionRequest':
          dispatch(addPermissionRequest(message.params));
          break;
        case 'documentCreated':
          dispatch(addDocument(message.params));
          break;
      }
    }
  }

  const handleVote = async (messageId: string, voteType: 'approve' | 'disapprove') => {
    try {
      await sendRequest(McpMethod.Vote, { messageId, voteType });
      
      const vote: Vote = {
        agentId: state.humanUserId,
        agentName: 'Human User',
        voteType,
        timestamp: Date.now(),
      };
      dispatch(updateMessageVotes({ messageId, vote }));
    } catch (error) {
      console.error('Failed to send vote:', error);
    }
  };

  const handleActionRequest = async (action: { type: 'request_counter_proposal' | 'state_reason'; targetAgentId: string; messageId: string }) => {
    try {
      if (action.type === 'request_counter_proposal') {
        await sendRequest(McpMethod.RequestCounterProposal, {
          messageId: action.messageId,
          targetAgentId: action.targetAgentId,
        });
      } else {
        await sendRequest(McpMethod.StateVoteReason, {
          messageId: action.messageId,
          targetAgentId: action.targetAgentId,
        });
      }
    } catch (error) {
      console.error('Failed to send action request:', error);
    }
  };

  const handleDocumentClick = async (documentId: string) => {
    try {
      const result = await sendRequest(McpMethod.GetDocument, { documentId });
      setSelectedDocument(result);
    } catch (error) {
      console.error('Failed to get document:', error);
    }
  };

  const handleSendMessage = async () => {
    if (!messageInput.trim()) return;

    try {
      await sendRequest(McpMethod.SendMessage, {
        content: messageInput,
        channelId: state.currentChannelId,
      });
      setMessageInput('');
    } catch (error) {
      console.error('Failed to send message:', error);
    }
  };

  const handleGrantPermission = async (requestId: string) => {
    try {
      await sendRequest('grantPermission', { requestId });
      dispatch(updatePermissionStatus({ requestId, status: 'granted' as any }));
    } catch (error) {
      console.error('Failed to grant permission:', error);
    }
  };

  const handleDenyPermission = async (requestId: string) => {
    try {
      await sendRequest('denyPermission', { requestId });
      dispatch(updatePermissionStatus({ requestId, status: 'denied' as any }));
    } catch (error) {
      console.error('Failed to deny permission:', error);
    }
  };

  const handlePMSendMessage = async (pmId: string, content: string) => {
    try {
      await sendRequest(McpMethod.SendMessage, { content, pmId, isPrivate: true });
      dispatch(addPMEntry({ pmId, entry: {
        id: `msg-${Date.now()}`,
        senderId: state.humanUserId,
        content,
        timestamp: Date.now(),
        type: 'message',
      }}));
    } catch (error) {
      console.error('Failed to send PM:', error);
    }
  };

  const handleSubmitWork = async (pmId: string) => {
    try {
      await sendRequest(McpMethod.Submit, { pmId });
      dispatch(addPMEntry({ pmId, entry: {
        id: `submit-${Date.now()}`,
        senderId: state.humanUserId,
        content: '/submit - Work submitted for review',
        timestamp: Date.now(),
        type: 'submit',
      }}));
    } catch (error) {
      console.error('Failed to submit work:', error);
    }
  };

  return (
    <div className="app">
      <header className="app-header">
        <h1>Multi-Agent Chat</h1>
        <div className="connection-status">
          <span className={`status-indicator ${isConnected ? 'connected' : 'disconnected'}`}></span>
          {isConnected ? 'Connected' : 'Disconnected'}
        </div>
      </header>

      <div className="main-content">
        <div className="chat-area">
          <div className="messages-container">
            {state.messages.map((message) => (
              <ChatMessageComponent
                key={message.id}
                message={message}
                onVote={handleVote}
                onActionRequest={handleActionRequest}
                onDocumentClick={handleDocumentClick}
              />
            ))}
          </div>

          <div className="message-input-area">
            <textarea
              value={messageInput}
              onChange={(e) => setMessageInput(e.target.value)}
              placeholder="Type your message..."
              rows={3}
            />
            <button onClick={handleSendMessage} disabled={!isConnected}>
              Send
            </button>
          </div>
        </div>

        <div className="pm-sidebar">
          <div className="pm-tabs-header">
            <h3>Private Messages</h3>
          </div>
          <div className="pm-tabs-list">
            {state.privateMessages.map((pm) => (
              <PMTab
                key={pm.id}
                pm={pm}
                isActive={state.activePMTab === pm.id}
                onActivate={() => dispatch(setActivePMTab(pm.id))}
                onClose={() => dispatch(setActivePMTab(null))}
                onGrantPermission={handleGrantPermission}
                onDenyPermission={handleDenyPermission}
                onSendMessage={handlePMSendMessage}
                onSubmitWork={handleSubmitWork}
              />
            ))}
          </div>
        </div>
      </div>

      {selectedDocument && (
        <DocumentViewer
          document={selectedDocument}
          onClose={() => setSelectedDocument(null)}
        />
      )}
    </div>
  );
};

export default App;
