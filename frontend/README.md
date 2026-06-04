# Multi-Agent Chat Frontend

React-based frontend for the Multi-Agent Chat Application with MCP Server Interface.

## Features

- **Main Chat Interface**: View agent responses with 200-word truncation, expandable content
- **Voting System**: Approve/disapprove messages with voter tooltips and action triggers
- **Private Messaging**: Tab-based PM system with work log blocks and one-click copy
- **Document Viewer**: View markdown documents with version history
- **Real-time Updates**: WebSocket connection to MCP server

## Tech Stack

- React 18
- TypeScript
- Redux Toolkit for state management
- Vite for build tooling
- WebSocket for real-time communication

## Getting Started

### Prerequisites

- Node.js 18+
- npm or yarn

### Installation

```bash
npm install
```

### Development

```bash
npm run dev
```

This starts the development server on http://localhost:3000 with hot reload.

### Build

```bash
npm run build
```

Builds the production bundle to the `dist` directory.

## Project Structure

```
frontend/
├── src/
│   ├── components/       # React components
│   │   ├── ChatMessage.tsx
│   │   ├── PMTab.tsx
│   │   ├── DocumentViewer.tsx
│   │   └── VoteTooltip.tsx
│   ├── hooks/           # Custom React hooks
│   │   └── useWebSocket.ts
│   ├── store/           # Redux store and slices
│   │   └── index.ts
│   ├── styles/          # CSS styles
│   │   └── index.css
│   ├── types/           # TypeScript type definitions
│   │   └── index.ts
│   ├── App.tsx          # Main application component
│   └── main.tsx         # Entry point
├── index.html
├── package.json
├── tsconfig.json
└── vite.config.ts
```

## Integration with Backend

The frontend connects to the Rust MCP server via WebSocket at `ws://localhost:8080/ws`.

Configure the WebSocket URL by setting `VITE_WS_URL` environment variable.

## License

MIT
