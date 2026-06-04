import { useState, useEffect, useCallback, useRef } from 'react';
import { McpMethod, WebSocketMessage } from '../types';

interface UseWebSocketOptions {
  url: string;
  onMessage?: (message: WebSocketMessage) => void;
  onError?: (error: Event) => void;
  onOpen?: () => void;
  onClose?: () => void;
}

export function useWebSocket({ url, onMessage, onError, onOpen, onClose }: UseWebSocketOptions) {
  const [isConnected, setIsConnected] = useState(false);
  const wsRef = useRef<WebSocket | null>(null);
  const messageHandlersRef = useRef<Map<string, (message: WebSocketMessage) => void>>(new Map());

  const connect = useCallback(() => {
    try {
      const ws = new WebSocket(url);

      ws.onopen = () => {
        setIsConnected(true);
        onOpen?.();
      };

      ws.onclose = () => {
        setIsConnected(false);
        onClose?.();
      };

      ws.onerror = (error) => {
        onError?.(error);
      };

      ws.onmessage = (event) => {
        try {
          const message: WebSocketMessage = JSON.parse(event.data);
          
          // Check if we have a handler for this message ID
          if (message.id) {
            const handler = messageHandlersRef.current.get(message.id);
            if (handler) {
              handler(message);
              messageHandlersRef.current.delete(message.id);
              return;
            }
          }
          
          onMessage?.(message);
        } catch (e) {
          console.error('Failed to parse WebSocket message:', e);
        }
      };

      wsRef.current = ws;
    } catch (error) {
      console.error('Failed to create WebSocket connection:', error);
    }
  }, [url, onMessage, onError, onOpen, onClose]);

  const disconnect = useCallback(() => {
    if (wsRef.current) {
      wsRef.current.close();
      wsRef.current = null;
      setIsConnected(false);
    }
  }, []);

  const send = useCallback((message: WebSocketMessage) => {
    if (wsRef.current && wsRef.current.readyState === WebSocket.OPEN) {
      wsRef.current.send(JSON.stringify(message));
      return true;
    }
    return false;
  }, []);

  const sendRequest = useCallback((method: McpMethod, params: any, timeoutMs: number = 30000): Promise<any> => {
    return new Promise((resolve, reject) => {
      const id = `req-${Date.now()}-${Math.random().toString(36).substr(2, 9)}`;
      
      const request: WebSocketMessage = {
        type: 'request',
        id,
        method,
        params,
      };

      const timeoutId = setTimeout(() => {
        messageHandlersRef.current.delete(id);
        reject(new Error(`Request timeout for ${method}`));
      }, timeoutMs);

      const messageHandler = (msg: WebSocketMessage) => {
        clearTimeout(timeoutId);
        if (msg.error) {
          reject(new Error(msg.error.message));
        } else {
          resolve(msg.result);
        }
      };

      messageHandlersRef.current.set(id, messageHandler);

      if (!send(request)) {
        clearTimeout(timeoutId);
        messageHandlersRef.current.delete(id);
        reject(new Error('Not connected'));
      }
    });
  }, [send]);

  useEffect(() => {
    connect();

    return () => {
      disconnect();
    };
  }, [connect, disconnect]);

  return {
    isConnected,
    send,
    sendRequest,
    connect,
    disconnect,
  };
}
