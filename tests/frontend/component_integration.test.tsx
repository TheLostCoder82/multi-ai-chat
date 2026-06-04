import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { ChatMessage } from '../components/ChatMessage';
import { VoteTooltip } from '../components/VoteTooltip';
import { PMTab } from '../components/PMTab';
import { DocumentViewer } from '../components/DocumentViewer';

describe('Frontend Integration Tests', () => {
  
  describe('ChatMessage Component', () => {
    
    it('should truncate long messages at 200 words', () => {
      const longContent = Array(300).fill('word').join(' ');
      
      render(<ChatMessage 
        id="msg1" 
        sender="agent1" 
        content={longContent} 
        timestamp={new Date()}
        votes={{ approve: 0, disapprove: 0 }}
      />);
      
      // Should show truncated version
      expect(screen.getByText(/word word word/)).toBeInTheDocument();
      // Should have expand button
      expect(screen.getByText('Show more')).toBeInTheDocument();
    });
    
    it('should display document links correctly', () => {
      const contentWithDoc = 'Check out doc-123 for details';
      
      render(<ChatMessage 
        id="msg1" 
        sender="agent1" 
        content={contentWithDoc} 
        timestamp={new Date()}
        votes={{ approve: 0, disapprove: 0 }}
      />);
      
      // Document link should be clickable
      const docLink = screen.getByText('doc-123');
      expect(docLink).toHaveAttribute('href');
    });
    
    it('should show vote counts and tooltip trigger', () => {
      render(<ChatMessage 
        id="msg1" 
        sender="agent1" 
        content="Test message" 
        timestamp={new Date()}
        votes={{ approve: 5, disapprove: 2 }}
      />);
      
      expect(screen.getByText('👍 5')).toBeInTheDocument();
      expect(screen.getByText('👎 2')).toBeInTheDocument();
    });
  });
  
  describe('VoteTooltip Component', () => {
    
    it('should display voter list on hover', () => {
      const voters = {
        approve: ['agent1', 'agent2', 'agent3'],
        disapprove: ['agent4']
      };
      
      render(<VoteTooltip voters={voters} />);
      
      // Trigger hover
      const tooltipTrigger = screen.getByTestId('vote-tooltip-trigger');
      fireEvent.mouseEnter(tooltipTrigger);
      
      waitFor(() => {
        expect(screen.getByText('agent1')).toBeInTheDocument();
        expect(screen.getByText('agent2')).toBeInTheDocument();
      });
    });
    
    it('should show counter-proposal action on rejection', () => {
      const voters = {
        approve: [],
        disapprove: ['agent1', 'agent2', 'agent3']
      };
      
      render(<VoteTooltip voters={voters} messageId="msg1" />);
      
      // Counter-proposal button should appear
      expect(screen.getByText('Create Counter-Proposal')).toBeInTheDocument();
    });
  });
  
  describe('PMTab Component', () => {
    
    it('should display work log with copy button', () => {
      const workLog = {
        agentId: 'agent1',
        tasks: ['Task 1', 'Task 2'],
        hours: 8
      };
      
      render(<PMTab currentAgent="admin" participant="agent1" workLog={workLog} />);
      
      expect(screen.getByText('Work Log')).toBeInTheDocument();
      expect(screen.getByText('Copy to Clipboard')).toBeInTheDocument();
    });
    
    it('should handle /submit command', () => {
      render(<PMTab currentAgent="agent1" participant="admin" />);
      
      const input = screen.getByPlaceholderText(/Type a message/);
      fireEvent.change(input, { target: { value: '/submit' } });
      fireEvent.keyDown(input, { key: 'Enter', code: 'Enter' });
      
      // Should submit work log
      waitFor(() => {
        expect(screen.getByText('Work log submitted')).toBeInTheDocument();
      });
    });
    
    it('should show grant/deny permission controls for admin', () => {
      render(<PMTab currentAgent="admin" participant="agent1" isAdmin={true} />);
      
      expect(screen.getByText('Grant Permission')).toBeInTheDocument();
      expect(screen.getByText('Deny Permission')).toBeInTheDocument();
    });
  });
  
  describe('DocumentViewer Component', () => {
    
    it('should render markdown content', () => {
      const doc = {
        id: 'doc1',
        title: 'Test Document',
        content: '# Heading\n\n**Bold text** and *italic*',
        version: 1
      };
      
      render(<DocumentViewer document={doc} />);
      
      expect(screen.getByText('Test Document')).toBeInTheDocument();
      // Markdown should be rendered
      expect(screen.getByRole('heading', { name: 'Heading' })).toBeInTheDocument();
    });
    
    it('should display version history', () => {
      const doc = {
        id: 'doc1',
        title: 'Test Document',
        content: 'Content',
        version: 3,
        history: [
          { version: 1, timestamp: new Date(), author: 'agent1' },
          { version: 2, timestamp: new Date(), author: 'agent2' },
          { version: 3, timestamp: new Date(), author: 'agent1' }
        ]
      };
      
      render(<DocumentViewer document={doc} />);
      
      expect(screen.getByText('Version History')).toBeInTheDocument();
      expect(screen.getAllByText(/Version \d/)).toHaveLength(3);
    });
    
    it('should support external editor export', () => {
      const doc = {
        id: 'doc1',
        title: 'Test Document',
        content: 'Content',
        version: 1
      };
      
      render(<DocumentViewer document={doc} />);
      
      const exportButton = screen.getByText('Edit Externally');
      fireEvent.click(exportButton);
      
      // Should trigger download or open in external editor
      waitFor(() => {
        expect(screen.getByText('Document exported')).toBeInTheDocument();
      });
    });
  });
});
