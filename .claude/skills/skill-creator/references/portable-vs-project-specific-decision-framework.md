<atom>
  <type>reference</type>
  <diataxis_quadrant>explanation</diataxis_quadrant>
  <intent>determine_portability_scope</intent>
</atom>

# Portable vs Project-Specific: Decision Framework

<nav>
  <tag name="core-principle" section="Core Principle" />
  <tag name="three-factor-test" section="Three-Factor Dependency Test" />
  <tag name="binary-checklist" section="Binary Decision Checklist" />
  <tag name="third-party-guidance" section="Third-Party Library Integration Guidance" />
  <tag name="few-shot-examples" section="Few-Shot Examples" />
  <tag name="placeholder-pattern" section="Placeholder Pattern" />
  <tag name="atomization-note" section="Note on Excessive Atomization" />
</nav>

## Core Principle

The distinction between **portable** and **project-specific** content hinges on **structural dependency**.

**Portable logic** stands alone without knowing your monorepo layout. It can be applied to any project using the same technology stack.

**Project-specific content** requires knowledge of your exact directory structure, package names, npm scripts, and CI/CD configuration.

---

## Three-Factor Dependency Test

Use this test to determine if content is project-specific:

| Factor                     | Portable                                              | Project-Specific                                                                               |
| -------------------------- | ----------------------------------------------------- | ---------------------------------------------------------------------------------------------- |
| **Path Dependency**        | Uses placeholders `{MAIN_APP_PATH}`, generic examples | References actual paths: `apps/identity-backend`, `packages/db`                                |
| **Package Dependency**     | Mentions libraries generically: "Use Drizzle"         | References specific package names: `pnpm --filter @identity-backend/db`                        |
| **Environment Dependency** | Abstract environment variables: `{DATABASE_URL}`      | Hardcoded `.env` values: `DATABASE_URL=postgresql://postgres:postgres@localhost:5432/identity` |

### Decision Matrix

| Test Result            | Action                                                       | Examples                                        |
| ---------------------- | ------------------------------------------------------------ | ----------------------------------------------- |
| **Fails ANY factor**   | Mark as project-specific (`compatibility: opencode-project`) | Real paths, scoped packages, hardcoded env vars |
| **Passes ALL factors** | Mark as portable (`compatibility: opencode`)                 | Placeholders, generic names, tech-agnostic      |

---

## Binary Decision Checklist

### Project-Specific Test

**If ANY answer is "Yes" → Project-Specific**:

- [ ] Does the content reference monorepo package names (e.g., `@my-org/package`)?
- [ ] Does it show specific directory structures unique to your project?
- [ ] Does it mention project-specific npm scripts not in generic templates?
- [ ] Does it assume specific environment variable names from your `.env.example`?
- [ ] Does it reference local file paths (`.claude/skills/`, `apps/`)?
- [ ] Does it require knowledge of your CI/CD configuration?

### Portable Test

**If ALL answers are "No" → Portable**:

- [ ] Could this guidance apply to any project using this tech stack?
- [ ] Are examples using `{PLACEHOLDER}` notation or generic names?
- [ ] Does it focus on methodology, patterns, or library usage?
- [ ] Would it work in a fresh repo with standard tooling?

---

## Third-Party Library Integration Guidance

### Portable (Library Knowledge)

Belongs in standalone skills with `compatibility: opencode`.

**Examples**:

- **Library deprecation notices**: "@effect/schema is deprecated, use @effect/schema/parse"
- **Framework best practices**: "Use Effect-TS services for dependency injection"
- **Integration patterns**: "How to use Playwright with Docker for cross-platform testing"
- **API usage patterns**: "Drizzle ORM migration workflow: generate → migrate → studio"
- **Methodological guidance**: "Always use parameterized queries to prevent SQL injection"

### Project-Specific (Your Implementation)

Belongs in `opencode-project` skills.

**Examples**:

- **Package references**: "Use `pnpm --filter @identity-backend/db db:migrate`"
- **Local setup**: "Run `pnpm --filter data-api-reverse-proxy dev` to start proxy"
- **Project conventions**: "All skills go in `.claude/skills/{skill-name}`"
- **Custom scripts**: "Use `pnpm test:staging` to run staging environment tests"
- **Local paths**: "See the README in `apps/data-api-reverse-proxy/README.md`"

### Atomization Rule

**For portable skills**: Each third-party library integration should be a separate atom or skill.

❌ **Anti-pattern**:

```markdown
## Third-Party Integrations

### Drizzle ORM

[50 lines of Drizzle content]

### Playwright

[30 lines of Playwright content]

### Effect-TS

[40 lines of Effect-TS content]
```

✅ **Correct**: Create separate atoms:

- `drizzle-orm-patterns.md`
- `playwright-testing.md`
- `effect-integration.md`

**For project-specific skills**: Consolidate all integrations into a single "Third-Party Integrations" section since project context makes everything related.

---

## Few-Shot Examples

### Example 1: Library Usage

**Portable** (skill: `drizzle-orm`):

```typescript
// Use parameterized queries to prevent SQL injection
const users = await db.select()
  .from(schema.users)
  .where(eq(schema.users.id, { USER_ID_PLACEHOLDER }))
```

**Project-Specific** (skill: `identity-backend`):

```typescript
// Query users in our system (packages/db schema)
import { db } from '@identity-backend/db'
import * as schema from '@identity-backend/db/schema'

const users = await db.select()
  .from(schema.users)
  .where(eq(schema.users.id, userId))
```

### Example 2: Build Commands

**Portable** (skill: `pnpm-workspaces`):

```bash
# Run commands in specific workspace
pnpm --filter {PACKAGE_NAME} {COMMAND}

# Example: Run tests
pnpm --filter {PACKAGE_NAME} test
```

**Project-Specific** (skill: `identity-backend`):

```bash
# Run tests for the database package
pnpm --filter @identity-backend/db test

# Run backend in development
pnpm --filter identity-backend-container dev
```

### Example 3: Directory Structure

**Portable** (skill: `monorepo-architecture`):

```yaml
{PROJECT_TYPE}/
├── {MAIN_APP_PATH}/
├── {PACKAGES_DIR}/
├── {CONFIG_PATH}/
└── {DOCS_PATH}/
```

**Project-Specific** (skill: `identity-backend`):

```yaml
identity-backend/
├── apps/
│   ├── identity-backend/
│   └── data-api-reverse-proxy/
├── packages/
│   ├── db/
│   ├── auth/
│   ├── rpc/
│   └── core/
└── .claude/skills/
```

### Example 4: Environment Configuration

**Portable** (skill: `nodejs-deployment`):

```bash
# Database configuration
DATABASE_URL={DATABASE_CONNECTION_STRING}

# App configuration
PORT={APP_PORT}
NODE_ENV={ENVIRONMENT}
```

**Project-Specific** (skill: `identity-backend`):

```bash
# Database (Docker Postgres)
DATABASE_URL=postgresql://postgres:postgres@localhost:5432/identity

# Network (Polkadot People chain)
PEOPLE_NETWORK=polkadot  # Options: polkadot, paseo, westend

# Security
ENFORCE_AUTH=true
PLAY_INTEGRITY_ENABLED=true
```

### Example 5: Testing Commands

**Portable** (skill: `vitest-patterns`):

```bash
# Run unit tests
pnpm test

# Run tests in watch mode
pnpm test:watch

# Run specific test pattern
pnpm test {TEST_PATTERN}
```

**Project-Specific** (skill: `identity-backend`):

```bash
# Run all workspace tests
pnpm test:workspace

# Run E2E tests with Docker
CI=true DEBUG=testcontainers* pnpm test:e2e

# Run staging environment tests
pnpm test:staging
```

### Example 6: Import Paths

**Portable** (skill: `typescript-imports`):

```typescript
// Import from local package
import { Component } from '{PACKAGE_NAME}'

// Import with path alias
import { utils } from '{PATH_ALIAS}/utils'
```

**Project-Specific** (skill: `identity-backend`):

```typescript
// Import from monorepo packages
import { auth } from '@identity-backend/auth'
import { db } from '@identity-backend/db'
import { rpc } from '@identity-backend/rpc'
```

### Example 7: Skill Metadata

**Portable** (skill: `effect-ts`):

```yaml
---
name: effect-ts
description: "Production Effect-TS patterns for functional programming"
license: MIT
compatibility: opencode  # Works in any Effect-TS project
---
```

**Project-Specific** (skill: `identity-backend`):

```yaml
---
name: identity-backend
description: "Polkadot App Backend development workflows"
license: MIT
compatibility: opencode-project  # Specific to identity-backend monorepo
---
```

---

## Placeholder Pattern (for Portable Skills)

Use placeholders in portable skill examples to ensure portability across projects:

```text
{PROJECT_TYPE}     → monorepo, single-package
{BUILD_TOOL}       → pnpm, npm, yarn
{MAIN_APP_PATH}    → apps/backend, src/
{PACKAGES_DIR}     → packages/
{DOCS_PATH}        → .claude/skills/, docs/
{OUTPUT_PATH}      → target file path
{PACKAGE_NAME}     → your-package-name
{USER_ID_PLACEHOLDER} → example-user-id
{DATABASE_CONNECTION_STRING} → postgresql://user:pass@host:port/db
{APP_PORT}         → 3000, 8080, etc.
{ENVIRONMENT}      → development, staging, production
```

**Rule**: Use placeholders in portable skills. Real references only in project-specific skills (`opencode-project` compatibility).

---

## Note on Excessive Atomization

The "excessive atomization" anti-pattern (>15 files, avg <150 lines) applies differently to portable vs project-specific skills.

### Portable Skills

**Atomize only when semantic domains are truly independent.**

✅ **Valid splits**:

- `effect-ts` (core concepts) vs `effect-ts-testing` (testing patterns)
- `terraform-cookbook` (infrastructure) vs `terraform-ci-cd` (pipelines)

❌ **Invalid splits** (excessive atomization):

- `effect-ts-error-handling` vs `effect-ts-async-errors` (too granular)
- `terraform-aws-s3` vs `terraform-aws-s3-buckets` (semantically overlapping)

### Project-Specific Skills

**Consolidate aggressively—project context makes everything related.**

✅ **Valid separate atoms**:

- Project setup/bootstrap (one-time tasks)
- Daily development workflows (common operations)
- Deployment procedures (infrequent but complex)
- Testing strategy (distinct from dev workflow)

❌ **Invalid splits** (excessive atomization):

- 15 files for individual npm script explanations
- One file per environment variable
- Separate files for "frontend" and "backend" when they share a monorepo

### Third-Party Integration Atomization

**For portable skills**: Each major library gets its own atom or skill.

- Valid: Separate `drizzle-orm.md`, `playwright-testing.md`, `effect-integration.md`
- Invalid: Single "third-party-integrations.md" file that lumps everything together

**Exception**: If libraries are semantically related (e.g., multiple AWS services), they can share a file like `aws-services-comprehensive.md`.

**For project-specific skills**: Consolidate all integrations into one "Third-Party Integrations" section since project context unifies them.

---

## See Also

- [Progressive Disclosure: Comprehensive Guide](progressive-disclosure-comprehensive-guide.md) - Tier 1/2/3 architecture
- [Validation: Quality Gates](validation-quality-gates.md) - SkillScore and rubric-based metrics
- [Workflow: S.T.A.R. Creation](creation-star-method.md) - Creating new skills with proper portability
