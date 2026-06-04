# Integration Tests

This directory contains integration tests for the Multi-Agent Chat Application.

## Test Categories

1. **Backend Integration Tests** (`backend/`)
   - Permission system integration
   - Message routing end-to-end
   - Document storage workflows
   - MCP server communication

2. **Frontend Integration Tests** (`frontend/`)
   - Component integration
   - State management flows
   - WebSocket communication

3. **End-to-End Tests** (`e2e/`)
   - Complete user workflows
   - Agent interaction scenarios

## Running Tests

```bash
# Backend integration tests
cargo test --test integration

# Frontend integration tests
npm run test:integration

# All tests
npm run test:all
```

## Test Coverage Goals

- Permission state transitions: 100%
- Message routing: 95%
- Document operations: 90%
- MCP protocol: 95%
- UI components: 85%
