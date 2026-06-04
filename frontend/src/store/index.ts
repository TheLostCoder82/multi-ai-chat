import { configureStore, createSlice, PayloadAction } from '@reduxjs/toolkit';
import {
  AppState,
  Agent,
  ChatMessage,
  Document,
  PrivateMessage,
  Vote,
  PermissionRequest,
  PermissionStatus,
  PMEntryType,
  WorkLogEntry,
} from '../types';

const initialState: AppState = {
  agents: [],
  messages: [],
  documents: [],
  privateMessages: [],
  activePMTab: null,
  currentChannelId: 'general',
  isConnected: false,
  humanUserId: 'human-user',
};

const appSlice = createSlice({
  name: 'app',
  initialState,
  reducers: {
    // Connection state
    setConnected(state, action: PayloadAction<boolean>) {
      state.isConnected = action.payload;
    },

    // Agents
    addAgent(state, action: PayloadAction<Agent>) {
      const existingIndex = state.agents.findIndex(a => a.id === action.payload.id);
      if (existingIndex >= 0) {
        state.agents[existingIndex] = action.payload;
      } else {
        state.agents.push(action.payload);
      }
    },

    removeAgent(state, action: PayloadAction<string>) {
      state.agents = state.agents.filter(a => a.id !== action.payload);
    },

    updateAgentStatus(state, action: PayloadAction<{ agentId: string; isConnected: boolean }>) {
      const agent = state.agents.find(a => a.id === action.payload.agentId);
      if (agent) {
        agent.isConnected = action.payload.isConnected;
      }
    },

    // Messages
    addMessage(state, action: PayloadAction<ChatMessage>) {
      state.messages.push(action.payload);
    },

    updateMessageVotes(state, action: PayloadAction<{ messageId: string; vote: Vote }>) {
      const message = state.messages.find(m => m.id === action.payload.messageId);
      if (message) {
        const { vote } = action.payload;
        if (vote.voteType === 'approve') {
          message.votes.approve.push(vote);
        } else {
          message.votes.disapprove.push(vote);
        }
      }
    },

    // Documents
    addDocument(state, action: PayloadAction<Document>) {
      state.documents.push(action.payload);
    },

    updateDocument(state, action: PayloadAction<Document>) {
      const index = state.documents.findIndex(d => d.id === action.payload.id);
      if (index >= 0) {
        state.documents[index] = action.payload;
      }
    },

    // Private Messages
    createPrivateMessage(state, action: PayloadAction<PrivateMessage>) {
      state.privateMessages.push(action.payload);
    },

    addPMEntry(state, action: PayloadAction<{ pmId: string; entry: any }>) {
      const pm = state.privateMessages.find(p => p.id === action.payload.pmId);
      if (pm) {
        pm.messages.push(action.payload.entry);
        pm.updatedAt = Date.now();
      }
    },

    addWorkLogEntry(state, action: PayloadAction<{ pmId: string; entry: WorkLogEntry }>) {
      const pm = state.privateMessages.find(p => p.id === action.payload.pmId);
      if (pm) {
        const lastEntry = pm.messages[pm.messages.length - 1];
        if (lastEntry && lastEntry.type === PMEntryType.WorkLog && lastEntry.workLogBlock) {
          lastEntry.workLogBlock.entries.push(action.payload.entry);
        } else {
          // Create new work log block
          pm.messages.push({
            id: `worklog-${Date.now()}`,
            senderId: action.payload.entry.type,
            content: '',
            timestamp: Date.now(),
            type: PMEntryType.WorkLog,
            workLogBlock: {
              entries: [action.payload.entry],
              copyable: true,
            },
          });
        }
        pm.updatedAt = Date.now();
      }
    },

    setActivePMTab(state, action: PayloadAction<string | null>) {
      state.activePMTab = action.payload;
    },

    // Permissions
    addPermissionRequest(state, action: PayloadAction<PermissionRequest>) {
      const pm = state.privateMessages.find(p => 
        p.participants.includes(action.payload.agentId)
      );
      if (pm) {
        pm.permissionRequests.push(action.payload);
      }
    },

    updatePermissionStatus(state, action: PayloadAction<{ requestId: string; status: PermissionStatus }>) {
      for (const pm of state.privateMessages) {
        const request = pm.permissionRequests.find(r => r.id === action.payload.requestId);
        if (request) {
          request.status = action.payload.status;
          break;
        }
      }
    },

    // Channel
    setCurrentChannel(state, action: PayloadAction<string>) {
      state.currentChannelId = action.payload;
    },
  },
});

export const {
  setConnected,
  addAgent,
  removeAgent,
  updateAgentStatus,
  addMessage,
  updateMessageVotes,
  addDocument,
  updateDocument,
  createPrivateMessage,
  addPMEntry,
  addWorkLogEntry,
  setActivePMTab,
  addPermissionRequest,
  updatePermissionStatus,
  setCurrentChannel,
} = appSlice.actions;

export const store = configureStore({
  reducer: {
    app: appSlice.reducer,
  },
});

export type AppDispatch = typeof store.dispatch;
export type RootState = ReturnType<typeof store.getState>;
