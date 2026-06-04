# Task: Rust-Based Multi-Agent Chat Application with MCP Server Interface

## Role
You are an expert Rust and System Architecture Engineer. Your goal is to design and plan a desktop chat application using Tauri that functions as an MCP (Model Context Protocol) Server. This application will allow a human user to host a chat room where multiple AI agents can connect, collaborate, and respond to the human user.

## Technical Specifications & Constraints
1.  **Core Architecture:**
    *   **Language:** Rust.
    *   **UI Framework:** Tauri (Native wrapper with web frontend).
    *   **Protocol:** The application must act as the **MCP Server**. Agents will connect to this application as clients.
    *   **Responsibility Separation:**
        *   **Agents:** Bring their own external tool access (e.g., separate worktrees, IDEs). They are responsible for execution.
        *   **Application:** Responsible for communication, coordination, and **Permission Management**. It does **not** execute code or access external projects directly.
    *   **Storage:** No shared workspace for agents within the app. All generated documents must be Markdown (.md) and stored locally within the app's data directory.

2.  **User Interface (Primary Purpose):**
    *   **Main Chat:**
        *   Displays agent responses to all participants.
        *   **Response Limits:** Agent snippets in the main chat must be limited to 200 words max.
        *   **Document Access:** Users must be able to click a link to read the full untruncated response and open any generated Markdown documents in a viewer.
        *   **Voting System:**
            *   Each agent response must include Approve/Disapprove links visible to other agents.
            *   **Display:** Show a count of votes (e.g., "👍 3 | 👎 1").
            *   **Tooltip:** On hover, show which specific agents voted which way.
            *   **Tooltip Actions:** Each agent name in the tooltip must have clickable links:
                *   "Request Counter-Proposal"
                *   "State Reason for Vote"
            *   Clicking these links triggers the specific agent to generate a new response.
    *   **Private Messaging (PM) System:**
        *   **Tabs:** The UI must support opening private message tabs between the Human and specific Agents.
        *   **Human Initiated:** Human can open a PM tab with any agent to pass instructions or grants of permission.
        *   **Agent Initiated:** Agents must open a PM tab (request) to ask for permission to perform external actions (e.g., "Request Permission to Modify File X").
        *   **Permission Controls:** PM interface must include explicit "Grant" / "Deny" controls for agent requests.
        *   **Log Portability:** The task work log within the PM tab must be aggregated into a **single block element** (e.g., a dedicated scrollable container) with a **one-click copy button**. This ensures the entire context can be instantly transferred to another agent for review.

3.  **Agent Behavior & Permission Workflow:**
    *   **Main Chat:** Read-only discussion, proposals, snippets, and voting. No external actions allowed from here.
    *   **Message Routing:**
        *   **General Discussion:** Routes to Main Chat.
        *   **Task Execution:** If an agent has active permission to perform a task, all thought processes, progress updates, and technical logs **must** be routed to the **Private Message tab** where permission was granted, specifically accumulating within the **single work log block element**. The Main Chat should not be cluttered with step-by-step execution details.
    *   **Permission Request:** If an agent deems an external action necessary, it **must** send a permission request via a Private Message tab.
    *   **Permission Lifecycle:**
        *   **Case-by-Case:** Permissions are granted for a specific task/action only. Even if access was granted for a file previously, a new request is required for a new task.
        *   **Persistence:** Once granted, permission persists *only* until the agent signals the task is complete.
        *   **Revocation:** When an agent submits their work for user review, their permissions must automatically revert to **Read-Only (0 permissions)**.
        *   **Completion Signal:** The agent must send a structured command (e.g., `/submit`) within the **same Private Message tab** where permission was granted. This command triggers the automatic revocation of permissions and notifies the user to review the work.
    *   **Execution:** Agents may only execute external actions (in their own worktrees) after receiving explicit permission via the PM system.
    *   **Reporting:** After execution and submission, agents should report a high-level summary back to the Main Chat if relevant to the group.

## Workflow & Consent Rules (CRITICAL)
You must follow this phased approach. Do not deviate from these rules.

1.  **Phase 1: Design & Recommendation**
    *   Analyze the requirements above.
    *   Create a detailed **Design Document** covering architecture, data flow, UI structure, and **Permission State Management** (including the state machine for permission lifecycle).
    *   **Crate Evaluation:** Specifically evaluate available Rust crates for implementing an MCP Server. Recommend the best option with pros/cons.
    *   **Security Review:** Explicitly confirm how the design prevents agents from accessing external data without user intervention.
    *   **STOP:** Do not write implementation code. Do not create an implementation plan yet.
    *   **Action:** Present the Design Document and Crate Recommendations. Then ask: **"Shall I proceed to Phase 2: Implementation Planning?"** and wait for my explicit consent.

2.  **Phase 2: Implementation Planning (Only after Consent)**
    *   Create a fine-grained, phased, step-by-step implementation plan.
    *   Break down the work into logical units (e.g., "Setup Tauri," "Implement MCP Server Core," "Build Voting Component," "Build PM Permission Flow").
    *   **STOP:** Do not write code yet.
    *   **Action:** Present the Plan. Then ask: **"Shall I proceed to Phase 3: Coding?"** and wait for my explicit consent.

3.  **Phase 3: Implementation (Only after Consent)**
    *   Begin coding based on the approved plan.

## Immediate Instruction
Start **Phase 1** now. Produce the Design Document and Crate Recommendations. Remember to **STOP** and ask for my consent before moving to Phase 2.
