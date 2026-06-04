/**
 * Core type definitions for the Multi-Agent Chat Application
 */

export interface Agent {
  id: string;
  name: string;
  capabilities: string[];
  isConnected: boolean;
}

export interface Vote {
  agentId: string;
  agentName: string;
  voteType: 'approve' | 'disapprove';
  reason?: string;
  timestamp: number;
}

export interface ChatMessage {
  id: string;
  senderId: string;
  senderName: string;
  content: string;
  timestamp: number;
  channelId: string;
  isTruncated: boolean;
  fullContent?: string;
  documentLinks?: DocumentLink[];
  votes: {
    approve: Vote[];
    disapprove: Vote[];
  };
  priority: MessagePriority;
}

export enum MessagePriority {
  Low = 'low',
  Normal = 'normal',
  High = 'high',
}

export interface DocumentLink {
  documentId: string;
  title: string;
  versionId?: string;
  uri: string;
}

export interface Document {
  id: string;
  title: string;
  authorId: string;
  authorName: string;
  currentVersionId: string;
  versions: DocumentVersion[];
  createdAt: number;
  updatedAt: number;
}

export interface DocumentVersion {
  id: string;
  content: string;
  createdAt: number;
  changeSummary?: string;
}

export interface PermissionRequest {
  id: string;
  agentId: string;
  agentName: string;
  scope: PermissionScope;
  action: string;
  status: PermissionStatus;
  requestedAt: number;
  expiresAt?: number;
}

export enum PermissionStatus {
  Pending = 'pending',
  Granted = 'granted',
  Denied = 'denied',
  Expired = 'expired',
  Revoked = 'revoked',
}

export interface PermissionScope {
  type: 'file' | 'directory' | 'command' | 'network';
  path?: string;
  command?: string;
  host?: string;
  port?: number;
}

export interface PrivateMessage {
  id: string;
  participants: string[]; // agent IDs
  messages: PMEntry[];
  permissionRequests: PermissionRequest[];
  isActive: boolean;
  createdAt: number;
  updatedAt: number;
}

export interface PMEntry {
  id: string;
  senderId: string;
  content: string;
  timestamp: number;
  type: PMEntryType;
  workLogBlock?: WorkLogBlock;
}

export enum PMEntryType {
  Message = 'message',
  PermissionRequest = 'permission_request',
  PermissionGrant = 'permission_grant',
  PermissionDeny = 'permission_deny',
  WorkLog = 'work_log',
  Submit = 'submit',
  System = 'system',
}

export interface WorkLogBlock {
  entries: WorkLogEntry[];
  copyable: boolean;
}

export interface WorkLogEntry {
  id: string;
  timestamp: number;
  content: string;
  type: 'thought' | 'progress' | 'technical' | 'summary';
}

export interface VoteTooltipAction {
  type: 'request_counter_proposal' | 'state_reason';
  targetAgentId: string;
  messageId: string;
}

export interface AppState {
  agents: Agent[];
  messages: ChatMessage[];
  documents: Document[];
  privateMessages: PrivateMessage[];
  activePMTab: string | null;
  currentChannelId: string;
  isConnected: boolean;
  humanUserId: string;
}

export interface WebSocketMessage {
  type: 'request' | 'response' | 'notification';
  id?: string;
  method?: string;
  params?: any;
  result?: any;
  error?: WebSocketError;
}

export interface WebSocketError {
  code: number;
  message: string;
  data?: any;
}

// MCP Protocol Methods
export enum McpMethod {
  Initialize = 'initialize',
  SendMessage = 'sendMessage',
  RequestPermission = 'requestPermission',
  Submit = 'submit',
  Vote = 'vote',
  RequestCounterProposal = 'requestCounterProposal',
  StateVoteReason = 'stateVoteReason',
  OpenPrivateChannel = 'openPrivateChannel',
  ClosePrivateChannel = 'closePrivateChannel',
  GetDocument = 'getDocument',
  ListDocuments = 'listDocuments',
  Ping = 'ping',
}
