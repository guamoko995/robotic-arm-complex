# Project Roadmap: Robotic Arm Complex

---

### Phase 1 — Firmware
- [ ] **Network Protocol Stabilization:**
    - Implement **Session Identifiers (Session ID)** to filter stale acknowledgments (Ack) during client reconnection.
- [ ] **State Management:**
    - Factory reset (Wi-Fi and system parameters) via a long press of the hardware button.
    - Implement **graceful shutdown** and client notification during radio mode switching (AP/STA).
    - Support for dynamic kinematics configuration (max speed, initial position) via control commands.
- [ ] **Indication:**
    - Implement hardware status indication (LED) to provide feedback on operating modes and connection state.

---

### Phase 2 — Client SDK & Utilities
- [ ] **Core SDK Refactoring:**
    - Implement **feedback-driven throttling** based on firmware acknowledgments.
    - Provide a synchronous, minimalist API for seamless integration into user applications.
- [ ] **Control CLI:**
    - Develop a command-line interface (CLI) on top of the SDK for manual control, configuration, and debugging.

---

### Phase 3 — High-level Logic ("The Brain")
- [ ] **Kinematics Engine:**
    - Host-side Forward and Inverse Kinematics (FK/IK) module.
    - Geometric constraint handling and trajectory planning.

---

### Phase 4 — Voice Interface & Integration
- [ ] **Voice Bridge:**
    - Client-side bridge for voice assistants (Google Assistant, Alexa, etc.).
    - High-level Natural Language Processing (NLP) to translate voice commands into robot action sequences.
