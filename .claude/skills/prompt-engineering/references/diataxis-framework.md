<atom>
  <type>explanation</type>
  <diataxis_quadrant>explanation</diataxis_quadrant>
  <intent>understand_diataxis_information_architecture</intent>
</atom>

# Diátaxis Framework: The Cognitive Map for AI Prompting

## Context

The Diátaxis framework, originally developed by Daniele Procida for technical documentation, serves as the "Operating System" of AI prompting. It defines the **persona** and **output mode** for LLM interactions. Most prompt failures stem from mode confusion—asking for a quick fix but receiving a lecture, or requesting a concept check and getting dry API parameters.

Derived from Ancient Greek _dia_ ("across") and _taxis_ ("arrangement"), Diátaxis maps all technical communication needs onto two intersecting axes: **Theory vs. Practice** and **Acquisition vs. Application**.

---

## The Two Cognitive Axes

### Axis 1: Theory vs. Practice (Content Focus)

- **Theory**: Abstract concepts, principles, understanding "why"
- **Practice**: Concrete steps, implementation, doing "how"

### Axis 2: Acquisition vs. Application (User Mindset)

- **Acquisition (Study)**: User wants to learn, build mental models
- **Application (Work)**: User wants to accomplish a specific task

---

## The Four Quadrants

| Diátaxis Mode    | Axis 1 (Focus) | Axis 2 (Action) | User Need              | AI Persona Archetype            | Key Characteristic                       |
| :--------------- | :------------- | :-------------- | :--------------------- | :------------------------------ | :--------------------------------------- |
| **Tutorial**     | Practice       | Acquisition     | "I want to learn"      | The Mentor / Teacher            | Learning-oriented, step-by-step pedagogy |
| **How-To Guide** | Practice       | Application     | "I want to do"         | The Senior Engineer / Colleague | Task-oriented, imperative, concise       |
| **Reference**    | Theory         | Application     | "I need facts"         | The Librarian / Database        | Information-oriented, structured, dry    |
| **Explanation**  | Theory         | Acquisition     | "I want to understand" | The Professor / Philosopher     | Context-oriented, discursive, generative |

---

## Quadrant 1: Tutorials (The Learning Experience)

### Philosophy

A Tutorial in Diátaxis is a **lesson**, not a task. The goal is learning, not completion. When designing a Tutorial prompt, the AI must inhibit its tendency to solve problems immediately and instead guide the user step-by-step.

### User State

- **Mindset**: Acquisition (Study)
- **Focus**: Practice
- **Expertise**: Novice to beginner
- **Goal**: Competence through guided learning

### Prompt Engineering Application

**The Tutorial Prompt Structure**:

```markdown
Role: You are an expert Technical Educator specializing in [domain].
Diátaxis Mode: Tutorial

Constraints:

- DO NOT provide the final answer immediately
- Explain the concept first, then ask the user to attempt the step
- Provide feedback, not solutions
- Use learning-oriented language ("Let's explore...", "Notice how...")
- Ensure immediate success at each step before proceeding
- No alternatives or branching paths (linear progression only)

Task: Teach [concept] step-by-step.

Format:

## Step 1: [Concept Name]

[Explanation with analogies]
[Practice exercise]
[Success confirmation]
```

**Example - Teaching Variables**:

```markdown
Role: You are a Python Educator.
Diátaxis Mode: Tutorial

Constraints:

- Guide the learner to discover variable concepts
- Don't just show code—explain what happens in memory
- Ask questions to check understanding

Task: Teach Python variables.

## Step 1: What is a Variable?

Think of a variable as a labeled box in computer memory.

- The box has a name (the variable name)
- The box holds something (the value)
- You can change what's in the box (reassignment)

Exercise: Create a variable named `age` and assign your age to it.
(When you're ready, I'll show you how and explain what happens in memory.)
```

### Cognitive Insight

Tutorial mode leverages the LLM's ability to simulate "Theory of Mind" for beginners, adjusting vocabulary complexity and pacing. The model must:

- Inhibit direct problem-solving impulses
- Simulate pedagogical scaffolding
- Adjust cognitive load gradually
- Provide reinforcement learning feedback

### What TO Include ✅

- Linear progression (each step builds on previous)
- Immediate success verification
- Single learning path (no alternatives)
- Practice exercises after each concept
- Encouraging, safe tone
- Learning checkpoints

### What NOT to Include ❌

- Multiple approaches or alternatives
- Edge cases or advanced topics
- "Just copy this code" solutions
- Reference tables or specifications
- Abstract theory without practice

---

## Quadrant 2: How-To Guides (The Execution Protocol)

### Philosophy

The How-To guide focuses on **application** in the **practical** domain. The user is competent but stuck. They don't need to know "why" the algorithm works, only "how" to implement it to solve a specific error.

### User State

- **Mindset**: Application (Work)
- **Focus**: Practice
- **Expertise**: Competent professional
- **Goal**: Solve a specific problem

### Prompt Engineering Application

**The How-To Prompt Structure**:

```markdown
Role: You are a pragmatic Senior Engineer in [domain].
Diátaxis Mode: How-To Guide

Constraints:

- Eliminate theoretical preamble
- Provide linear sequence of actions
- Use imperative verbs ("Run this", "Check that")
- Assume domain knowledge
- No explanations of "why"—only "how"
- Allow conditional logic (IF X, THEN Y)

Task: Provide solution steps for [specific problem].

Format:

## Prerequisites

[What they need before starting]

## Solution

Step 1: [Direct action]
Step 2: [Direct action]

## Troubleshooting

IF [symptom]: [remediation]
IF [other symptom]: [remediation]
```

**Example - Fixing Database Connection**:

````markdown
Role: You are a Senior PostgreSQL Engineer.
Diátaxis Mode: How-To Guide

Constraints:

- No theory about connection pooling
- Direct troubleshooting steps only
- Assume user knows SQL

Task: Fix "connection refused" error in PostgreSQL 14.

## Prerequisites

- PostgreSQL 14+ installed
- Access to server logs
- pg_hba.conf file access

## Solution

Step 1: Check if server is running

```bash
sudo systemctl status postgresql
```
````

If inactive: `sudo systemctl start postgresql`

Step 2: Verify correct port

```bash
sudo netstat -tlnp | grep 5432
```

If missing: Edit `/etc/postgresql/14/main/postgresql.conf`, set `port = 5432`

Step 3: Check pg_hba.conf authentication

```bash
sudo cat /etc/postgresql/14/main/pg_hba.conf | grep -v '^#' | grep -v '^$'
```

Look for `host    all    all    127.0.0.1/32    scram-sha-256`

## Troubleshooting

IF "permission denied for database":
→ Check user exists: `\du` in psql
→ Create if missing: `CREATE USER your_user WITH PASSWORD 'password';`

IF "connection timeout":
→ Check firewall: `sudo ufw status`
→ Allow port: `sudo ufw allow 5432/tcp`

````
### What TO Include ✅

- Problem-oriented structure
- Branching logic (IF X, THEN Y)
- Prerequisites clearly stated
- Exact commands or steps
- Troubleshooting section
- Professional, austere tone

### What NOT to Include ❌

- Theoretical explanations ("The reason this works is...")
- Basic concept definitions ("A database is...")
- Multiple solution approaches (pick ONE working method)
- Pedagogical tone or encouragement
- Background history

---

## Quadrant 3: Reference (The Map of Territory)

### Philosophy

Reference documentation provides technical descriptions and factual information. It is **information-oriented** and **theoretical**. In AI terms, this is the "Retrieval" or "Specification" mode where the LLM acts as a structured database.

### User State

- **Mindset**: Application (Work)
- **Focus**: Theory
- **Expertise**: Professional scanning for information
- **Goal**: Retrieve facts quickly

### Prompt Engineering Application

**The Reference Prompt Structure**:

```markdown
Role: You are a Technical Documentarian and Database Archivist.
Diátaxis Mode: Reference

Constraints:
  - Output strict JSON or tabular data
  - No opinions or recommendations
  - No instructional steps (no "how to")
  - Objective facts only
  - Exhaustive coverage within scope
  - Temperature ~0 (minimal creativity)

Task: Provide reference documentation for [topic].

Format: Markdown tables with hierarchical organization.

Table Structure:
| Parameter | Type | Required | Description | Default | Constraints |
|-----------|------|----------|-------------|---------|-------------|
````

**Example - API Reference**:

```markdown
Role: You are an API Documentarian.
Diátaxis Mode: Reference

Constraints:

- Tables only
- No examples unless in table cells
- Objective specifications

Task: Document the GitHub REST API for repositories.

## Repository Endpoints

| Method | Endpoint                    | Auth Required | Parameters                | Description            | Rate Limit |
| ------ | --------------------------- | ------------- | ------------------------- | ---------------------- | ---------- |
| GET    | /repos/{owner}/{repo}       | Yes/No        | owner, repo               | Get repository details | 60/hour    |
| POST   | /repos/{owner}/{repo}/forks | Yes           | owner, repo, organization | Create a fork          | 60/hour    |
| DELETE | /repos/{owner}/{repo}       | Yes           | owner, repo               | Delete a repository    | 60/hour    |

## Response Schema

| Field     | Type    | Description          | Example               |
| --------- | ------- | -------------------- | --------------------- |
| id        | integer | Unique repository ID | 1296269               |
| name      | string  | Repository name      | "hello-world"         |
| full_name | string  | Owner/repo           | "octocat/hello-world" |
| private   | boolean | Visibility status    | false                 |
```

### What TO Include ✅

- Markdown tables for structured data
- Hierarchical organization for scanning
- Complete factual specifications
- Type signatures and parameter descriptions
- Cross-references between related entities
- Exhaustive coverage within domain

### What NOT to Include ❌

- Instructions or "how to" steps
- Opinions ("I recommend using...")
- Narrative explanations
- Examples outside of table cells
- "Why" something works

---

## Quadrant 4: Explanation (The Contextual Web)

### Philosophy

Explanation illuminates topics broadly, providing background and context. This is the most **generative** mode, where the AI's creativity can be channeled into useful analogies, synthesis, and connecting concepts.

### User State

- **Mindset**: Acquisition (Study)
- **Focus**: Theory
- **Expertise**: Learner wanting depth
- **Goal**: Understand "why" and "how it works"

### Prompt Engineering Application

**The Explanation Prompt Structure**:

```markdown
Role: You are a Subject Matter Expert and Professor of [domain].
Diátaxis Mode: Explanation

Constraints:

- Discuss history and context
- Connect X to Y (show relationships)
- Explore implications and design decisions
- Use narrative structure, not steps
- No implementation instructions
- Focus on understanding, not doing

Task: Explain [concept] in depth.

Format: Narrative prose (~800-1000 words)

Structure:

## Historical Context

[How the concept evolved]

## Theoretical Foundation

[Underlying principles and mental models]

## Connections

[How this relates to other concepts]

## Implications

[Real-world consequences and future directions]
```

**Example - Explaining Microservices**:

```markdown
Role: You are a Software Architecture Professor.
Diátaxis Mode: Explanation

Task: Explain microservices architecture.

## Historical Context

Microservices architecture emerged in the early 2010s as a response to the limitations of monolithic systems. Companies like Netflix and Amazon, struggling with deploying single codebases comprising millions of lines, pioneered the approach of breaking applications into small, independently deployable services.

## Theoretical Foundation

At its core, microservices embodies two principles:

1. **Single Responsibility**: Each service does one thing well (e.g., User Service handles ONLY user data)
2. **Decentralized Governance**: Teams own their services end-to-end, from code to deployment

This contrasts with monolithic architecture where:

- All functionality lives in one codebase
- Deployment requires redeploying the entire application
- A bug in the payment module can break the search feature

## Connections

Microservices connects to several architectural concepts:

- **SOA (Service-Oriented Architecture)**: Microservices is SOA with stricter constraints (smaller services, independent datastores)
- **Domain-Driven Design**: Service boundaries align with business domains
- **CAP Theorem**: Distributed microservices must trade off consistency vs availability

## Implications

The shift to microservices transforms organizations:

**Benefits**:

- Teams deploy independently (10 deploys/day vs 1 deploy/week)
- Fault isolation (payment service down doesn't break search)
- Technology diversity (payment in Java, search in Python)

**Costs**:

- Network complexity (service-to-service communication)
- Data consistency challenges (distributed transactions)
- Operational overhead (monitoring 50 services vs 1 monolith)
```

### What TO Include ✅

- Historical context and evolution
- Theoretical framework and mental models
- Connections between related concepts
- Design decisions and trade-offs
- Narrative structure
- Analogies and metaphors
- Future implications

### What NOT to Include ❌

- Step-by-step instructions ("Do this, then that")
- Code blocks (unless illustrative of concept)
- Implementation details
- "How to" accomplish tasks
- Quick fixes or solutions

---

## Adaptive Mode: The Dynamic Router

The most advanced application of Diátaxis is **Adaptive Mode**—where the prompt identifies the user's intent and switches quadrants dynamically.

### Implementation: Router Logic

```markdown
## ADAPTIVE DIÁTAXIS MODE

Analyze the user's input to determine cognitive state:

IF the user asks "How do I..." → Enter **HOW-TO** Mode

- User wants action (Application)
- Focus on practice (direct steps)
- Persona: Senior Engineer
- Constraints: Concise, imperative, no theory

IF the user asks "What is..." or "Why..." → Enter **EXPLANATION** Mode

- User wants understanding (Acquisition)
- Focus on theory (context)
- Persona: Professor
- Constraints: Narrative, connections, no steps

IF the user asks "Teach me..." → Enter **TUTORIAL** Mode

- User wants to learn (Acquisition)
- Focus on practice (guided exercises)
- Persona: Mentor
- Constraints: Linear, pedagogical, no alternatives

IF the user asks "List the..." or "Show me parameters..." → Enter **REFERENCE** Mode

- User needs facts (Application)
- Focus on theory (specifications)
- Persona: Librarian
- Constraints: Tables, no instructions, objective
```

### Example: Adaptive Prompt in Action

**User asks**: "I need to understand JWT tokens"

**Router Analysis**:

- "Understand" = Explanation mode (Acquisition + Theory)
- Switch to Professor persona
- Provide context, history, connections

**User asks**: "How do I implement JWT authentication?"

**Router Analysis**:

- "How do I" = How-to mode (Application + Practice)
- Switch to Senior Engineer persona
- Provide direct steps, commands, no theory

**User asks**: "Teach me JWT step by step"

**Router Analysis**:

- "Teach me" = Tutorial mode (Acquisition + Practice)
- Switch to Mentor persona
- Guide through exercises, check understanding

---

## Common Prompt Failures: Mode Confusion

### Failure 1: Mixed Tutorial + Reference

**Bad Prompt**:

```
"Teach me Python functions. Here's a reference table of all built-in functions..."
```

**Why it fails**: Beginner is overwhelmed by reference table; expert can't quickly find facts.

**Fix**: Split into separate documents

- `tutorial-python-functions.md` (Tutorial mode)
- `reference-python-functions.md` (Reference mode)

### Failure 2: Mixed How-To + Explanation

**Bad Prompt**:

```
"How do I fix this error? The reason this error occurs is..."
```

**Why it fails**: User wants to fix the problem, not learn history.

**Fix**: Separate modes

- `how-to-fix-error.md` (Pure solution steps)
- `explanation-error-causes.md` (Deep dive on why)

### Failure 3: Wrong Mode for User Expertise

**Bad Prompt**: Explaining basic concepts to expert user

**Why it fails**: Expert is frustrated by elementary explanations.

**Fix**: Router detects expertise (via user language) and switches to Reference or How-To mode accordingly.

---

## Integration with C.L.E.A.R. Framework

Diátaxis informs the C.L.E.A.R. components:

**For Tutorials**:

- **Role**: Technical Educator
- **Audience**: Novice (simple vocabulary)
- **Length**: Short sections with checkpoints
- **Examples**: Progressive difficulty
- **Context**: Learning-oriented

**For How-To Guides**:

- **Role**: Senior Engineer
- **Audience**: Expert (domain knowledge assumed)
- **Length**: As short as possible
- **Examples**: Troubleshooting scenarios
- **Context**: Problem-oriented

**For Reference**:

- **Role**: Technical Documentarian
- **Audience**: Professional (scanning)
- **Length**: Table format
- **Examples**: N/A (uses tables)
- **Context**: Information retrieval

**For Explanation**:

- **Role**: Subject Matter Expert
- **Audience**: Academic/Learner (depth)
- **Length**: Comprehensive (800-1000 words)
- **Examples**: Illustrative analogies
- **Context**: Conceptual understanding
