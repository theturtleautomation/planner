# Architecture Decision & BOM Tools: Research Findings

**Date:** March 5, 2026  
**Scope:** Tools managing software project "bill of materials" — the complete inventory of decisions, technologies, components, patterns, constraints, and quality attributes for a software project.

---

## Table of Contents

1. [Architecture Decision Records (ADR) Tools](#1-architecture-decision-records-adr-tools)
2. [Software Bill of Materials (SBOM) Tools](#2-software-bill-of-materials-sbom-tools)
3. [Architecture Documentation Platforms](#3-architecture-documentation-platforms)
4. [Technology Radar Tools](#4-technology-radar-tools)
5. [Developer Portals / Service Catalogs](#5-developer-portals--service-catalogs)
6. [Cross-Category Analysis: Patterns for a Living Architecture Blueprint](#6-cross-category-analysis-patterns-for-a-living-architecture-blueprint)

---

## 1. Architecture Decision Records (ADR) Tools

ADR tools manage the log of architectural decisions made on a project. The core discipline comes from Michael Nygard (2011): each decision is captured in a short, immutable document; only its **status** can change over time.

### ADR Status Lifecycle (common across tools)

```
Proposed → Accepted → (Deprecated | Superseded by ADR-NNN)
                                         ↓
                                   New ADR created
                                   references old one
```

---

### 1.1 ADR Tools (npryce/adr-tools)

**Source:** [github.com/npryce/adr-tools](https://github.com/npryce/adr-tools)  
**Type:** CLI tool (shell scripts). 5,200+ GitHub stars.

**Key Features:**
- Creates sequentially numbered Markdown ADR files (e.g., `0042-use-rust-for-critical-path.md`) in `doc/adr/` by default
- `adr new "Title"` — creates a new numbered ADR and opens in `$EDITOR`
- `adr new -s 9 "Title"` — creates a superseding ADR, automatically updates ADR-9's status to "Superseded by ADR-NNN"
- `adr generate toc` — generates a Markdown table of contents from all ADRs
- `adr generate graph` — generates a DOT-format dependency graph of ADR relationships
- `adr link` — manually creates explicit links between ADRs (e.g., "Amends", "Amended by")
- `adr list` — prints paths to all ADRs
- Template: Nygard's original (Title / Status / Context / Decision / Consequences)

**Decision History Management:**
- Bidirectional: superseding an ADR automatically annotates both the old and new files
- Superseded ADRs are preserved intact — only a "Superseded by" status line is added
- No UI; all navigation is via raw Markdown files or generated TOC

**CRUD / Editing:**
- Create: CLI command
- Read: raw Markdown files or generated static site
- Update: manual Markdown editing (only status changes intended)
- Delete: not supported (immutability by design)
- No web UI for editing

**Strengths for a planner-style tool:**
- Minimal, zero-dependency model — ADRs as first-class files in the repo
- Bidirectional supersession tracking is automatic
- DOT graph generation reveals ADR relationship topology
- Template-agnostic: can be adapted to any organization's format

**Limitations:**
- No visualization, search, or web UI out of the box
- No multi-repo or multi-package support
- Aging codebase (last release 2018)

---

### 1.2 Log4brains

**Source:** [github.com/thomvaill/log4brains](https://github.com/thomvaill/log4brains)  
**Type:** Node.js CLI + static site generator. Docs-as-code.

**Key Features:**
- `log4brains init` — interactive project setup, creates template and first ADR
- `log4brains adr new` — interactive CLI wizard to create a new ADR
- `log4brains preview` — local web UI with **hot reload** (live preview as you write Markdown)
- `log4brains build` — generates a static website deployable to GitHub Pages, GitLab Pages, S3
- **Timeline view** in the UI showing chronological decision history
- **Full-text search** across all ADRs in the static site
- ADR metadata automatically inferred from raw text and git log (dates, authors)
- **No enforced file numbering** — uses date-prefixed slugs (e.g., `20240315-use-kafka.md`) to avoid git merge conflicts
- Supports **multi-package monorepos**: global ADRs + package-specific ADRs, each with their own folder
- Customizable templates (default: MADR — Markdown Any Decision Records)
- CI/CD examples for auto-publish on merge

**Decision History Management:**
- Timeline menu shows all ADRs chronologically
- Superseded status is visible in the UI with links to the superseding decision
- Git history provides authorship context
- Roadmapped but not yet released: decision backlog, `@adr` code annotation, RSS feed, UI-based ADR creation/editing

**Unique Features:**
- Hot reload during authoring is a significant UX differentiator
- Metadata inference from git logs means no manual date tracking
- Multi-package support is ideal for microservices-scale projects

**CRUD / Editing:**
- Create: via CLI wizard
- Read: local hot-reload preview or deployed static site
- Update: Markdown editor of choice; preview refreshes instantly
- Delete: manual file deletion

**What makes UX effective:**
- The static site "feels like" a proper knowledge base: search, timeline, status badges, cross-links
- Zero infrastructure overhead — just files in git

---

### 1.3 Backstage ADR Plugin

**Source:** [backstage.io](https://backstage.io), [roadie.io/backstage/plugins/architecture-decision-records/](https://roadie.io/backstage/plugins/architecture-decision-records/)  
**Type:** Backstage plugin. Integrates ADRs into the service catalog entity pages.

**Key Features:**
- Adds an **ADRs tab** to any Backstage entity page (component, service, system)
- Reads ADR Markdown files from the entity's Git repository
- Supports **MADR v2 and v3** formats, plus Nygard format via custom parser
- Rendered Markdown with rewritten relative links and image paths (works for private repos via backend proxy)
- **Full-text search** across the entire Backstage catalog including ADRs — engineers can find decisions across all services
- Custom `filePathFilterFn` to include/exclude specific files
- `contentDecorators` pipeline: rewrite links, embed images, inject custom banners, format front matter
- Backend extension point for registering custom ADR parsers for non-standard formats
- Entity annotation: `backstage.io/adr-location: docs/adrs` maps the repo path

**Decision History Management:**
- ADR list shows title, status, and date inline
- Each ADR opens as rendered Markdown
- Cross-entity discovery via catalog-wide search

**Unique Features:**
- **Contextual placement**: ADRs live *next to* the service they affect, not in a separate knowledge base
- Cross-catalog search means finding "all decisions related to database choice" across 100+ services is feasible
- Ownership model: the team that owns the service maintains its ADRs via the same Git workflow

**CRUD / Editing:**
- Create: external (Markdown files in Git)
- Read: rendered inline in Backstage UI
- No in-UI editing

---

### 1.4 Arachne (arachne-framework)

**Source:** [github.com/arachne-framework/architecture](https://github.com/arachne-framework/architecture)  
**Type:** Not a dedicated tool — Arachne is an open-source Clojure web framework that uses ADRs as its primary governance document format.

**Key Insight:**
Arachne is significant not as a tool but as a **workflow model**:
- Every architectural decision requires an ADR *before* implementing code in `master`
- ADR workflow: Proposed → Accepted/Rejected (via steering group discussion)
- Supersession: new ADR references old one; old ADR gets an annotation pointing to the new one; old ADR is otherwise preserved unchanged
- Decisions stored as plain Markdown in `/architecture/adr-NNN-*.md`
- Collaborative review via pull requests

**What makes it notable for planner-style tools:**
- The "ADR as gate" pattern: code cannot be merged until the ADR is accepted
- Explicit steering group + voting process for contested decisions
- The decision record serves as a *constitutional document* for the codebase

---

### ADR Tools — Cross-Tool Feature Comparison

| Feature | adr-tools | Log4brains | Backstage ADR | Arachne |
|---|---|---|---|---|
| CLI creation | ✓ | ✓ | ✗ | ✗ |
| Web UI | ✗ | ✓ (static) | ✓ (Backstage) | ✗ |
| Hot reload preview | ✗ | ✓ | ✗ | ✗ |
| Full-text search | ✗ | ✓ | ✓ (catalog-wide) | ✗ |
| Timeline view | ✗ | ✓ | ✗ | ✗ |
| Supersession tracking | ✓ (auto) | ✓ | ✓ (rendered) | ✓ (manual) |
| Multi-package support | ✗ | ✓ | ✓ (per entity) | ✗ |
| Git-native | ✓ | ✓ | ✓ | ✓ |
| Relationship graph | ✓ (DOT) | ✗ | ✗ | ✗ |
| In-UI editing | ✗ | Roadmap | ✗ | ✗ |

---

### ADR Category Insights for a Living Architecture Blueprint

1. **Most impactful feature**: Contextual attachment of ADRs to the components they affect (Backstage model) — not a separate document store
2. **Critical UX pattern**: Immutability + supersession chain — readers can trace the full evolution of a decision
3. **Gap across all tools**: No in-UI CRUD for ADRs — authoring always requires external Markdown editing
4. **Pattern library angle**: ADRs function as a "decision ledger" for the BOM — each technology choice, pattern selection, or constraint has a corresponding ADR explaining *why*
5. **Planner-tool opportunity**: First-class decision status (Proposed/Accepted/Superseded) as a typed field, with automatic cross-linking when a new decision supersedes an old one

---

## 2. Software Bill of Materials (SBOM) Tools

SBOMs provide a formal, machine-readable inventory of software components — analogous to an ingredient list for software. Driven by regulatory requirements (US EO 14028, EU CRA) and supply chain security needs.

---

### 2.1 CycloneDX

**Source:** [cyclonedx.org/specification/overview/](https://cyclonedx.org/specification/overview/)  
**Standard body:** OWASP Foundation  
**Current version:** 1.7 (2025)

**Component Types Supported:**
- Software (libraries, frameworks, applications)
- Hardware devices
- Machine learning models
- Source code and configurations
- Cryptographic assets (CBOM — since v1.6)
- Services (external APIs with endpoint URIs, authentication info)

**Root-Level BOM Elements:**
| Element | Description |
|---|---|
| `metadata` | Supplier, manufacturer, tools used, BOM license |
| `components` | Full inventory of first/third-party components (name, version, PURL, hashes, licenses) |
| `services` | External APIs the software calls (endpoint URIs, auth requirements, trust boundaries) |
| `dependencies` | Dependency graph — what depends on what, direct and transitive |
| `compositions` | Completeness assertions (how much of the BOM is known vs. unknown) |
| `vulnerabilities` | Known CVEs, severity scores, VEX data, remediation status |
| `definitions` | Standards, best practices, and maturity models (v1.6+) |
| `citations` | Provenance of where BOM data originated (v1.7+) |

**Key Metadata Fields per Component:**
- `name`, `version`, `type` (library/framework/application/device/etc.)
- `purl` (Package URL — standard identifier)
- `cpe` (Common Platform Enumeration)
- `hashes` (MD5, SHA-1, SHA-256, SHA-512, Blake2b, etc.)
- `licenses` (SPDX expressions or custom)
- `supplier` / `manufacturer` / `author`
- `description`
- `externalReferences` (source repo, build system, issue tracker, etc.)
- `pedigree` (component's ancestry and provenance chain)

**Dependency Tracking:**
- Explicit dependency graph with `bom-ref` identifiers
- Captures direct and transitive dependencies
- Component-to-component and component-to-service relationships
- `compositions` element marks known-complete vs. known-incomplete inventory sections

**Vulnerability Features:**
- `vulnerabilities` section: CVE ID, source, ratings (CVSS, OWASP), affected versions, recommendations
- **VEX** (Vulnerability Exploitability eXchange) integration: mark vulns as "not affected" with justification
- Supports previously unknown vulnerabilities
- Exploitability context for each vulnerability

**Formats:** JSON, XML, Protobuf  
**License:** Apache 2.0

**What makes CycloneDX unique:**
- Purpose-built for security use cases (OWASP-governed)
- Broadest coverage of BOM *types*: Software + Hardware + ML + Crypto in one spec
- `services` element captures runtime API dependencies (not just package dependencies)
- `formulations` element captures how the software was built (reproducible builds)
- Extensible without breaking interoperability
- Strong VEX integration for vulnerability triage

---

### 2.2 SPDX

**Source:** [spdx.dev](https://spdx.dev/about/overview/)  
**Standard body:** Linux Foundation  
**ISO standard:** ISO/IEC 5962:2021  
**Current version:** 3.0.1 (2024)

**Scope (broader than CycloneDX):**
SPDX 3.0 expanded from software-only to a general system description standard with modular **profiles**:

| Profile | Purpose |
|---|---|
| Core | Base — mandatory for all |
| Software | Packages, files, snippets, build info |
| Security | Vulnerabilities, VEX, CVSS |
| Licensing | SPDX License List (690+ licenses), expressions |
| AI | AI models, training datasets, provenance |
| Dataset | Dataset metadata (for AI/ML) |
| Build | Build process, inputs, outputs, environments |
| Lite | Minimal license compliance subset |
| Extension | Custom extensions without breaking interoperability |

**Key Metadata Fields:**
- Package: `name`, `version`, `SPDXID`, `downloadLocation`, `filesAnalyzed`, `licenseConcluded`, `licenseDeclared`, `copyrightText`, `externalRefs` (PURL, CPE)
- Relationships: explicit `DEPENDS_ON`, `CONTAINS`, `DESCRIBES`, `GENERATED_FROM`, `BUILT_FROM` etc.
- Checksums: SHA1, SHA256, MD5, SHA512, Blake2b, etc.
- `snippet`: for partial-file licensing (e.g., a function copied from another project)

**Data Model:** Based on RDF; serialized as JSON-LD, Turtle, N-Triples, RDF/XML, tag-value

**License List:** 690+ curated SPDX license identifiers — canonical source for license data  
**SPDX license expressions:** `Apache-2.0 AND MIT`, `GPL-2.0-only OR MIT`

**Key Differentiators from CycloneDX:**
- Older and more license-compliance-focused; CycloneDX more security-focused
- SPDX License List is the canonical reference used by both specs
- File-level granularity (individual files and snippets, not just packages)
- ISO standard — required by some regulatory frameworks
- SPDX 3.0 now covers AI/ML, bringing it toward parity with CycloneDX's scope

---

### 2.3 Syft (Anchore)

**Source:** [github.com/anchore/syft](https://github.com/anchore/syft)  
**Type:** Open-source CLI tool and Go library  
**License:** Apache 2.0

**Key Features:**
- Generates SBOMs from **container images** (Docker, OCI, Singularity), **filesystems**, **archives**, OCI registries (without Docker daemon)
- Output formats: CycloneDX (XML + JSON), SPDX (Tag-Value + JSON), Syft JSON (lossless), GitHub JSON, Syft table
- Supports **dozens of package ecosystems**: apk, rpm, deb, npm, pip, gem, cargo, go modules, nuget, conan, swift, elixir, erlang, julia, dart, haskell, PHP (composer), R, etc.
- Linux distribution identification (detects Alpine, Debian, Ubuntu, RHEL, etc.)
- **Signed SBOM attestations** using the in-toto specification
- **Format conversion** between SBOM formats (`syft convert`)
- Pairs with **Grype** (also Anchore) for vulnerability scanning against generated SBOMs
- `--scope all-layers` to include packages from all image layers (not just final layer)
- Configuration via `.syft.yaml`
- Connects to private OCI registries
- `--file path/to/file` to save output instead of stdout

**Ecosystem detection depth:**
Syft's differentiator is detection *depth* — it doesn't just read package manager manifests but also inspects binary files and installed package databases, finding packages that manifest-only scanners miss.

**CRUD / Editing:**
- Generate: `syft <source> -o <format>`
- Output is a static file — no in-tool editing
- SBOM management at scale requires an enterprise platform (Anchore Enterprise)

---

### 2.4 FOSSA

**Source:** [fossa.com](https://fossa.com)  
**Type:** Commercial SCA + SBOM management platform

**Key Features:**
- **Software Composition Analysis (SCA)**: scans source code, container images, and binary files for open-source components
- **Binary Composition Analysis**: decomposes pre-built binaries (`.exe`, `.dll`, `.jar`, ELF, etc.) to detect embedded components
- **SBOM Generation**: produces CycloneDX or SPDX SBOMs in JSON/XML with one click; customizable fields
- **SBOM Ingestion**: ingest supplier-provided SBOMs; verify against binary scan results; compare for gaps
- **VEX Statements**: automatically populates Vulnerability Exploitability eXchange statements for customer distribution
- **License compliance engine**: expert-curated policy templates; automated approve/flag/deny by license type
- **Vulnerability prioritization**: CVSS + EPSS + CISA KEV + proprietary "remediation efficiency" metric
- **CI/CD integration**: GitHub, GitLab, Jenkins, CircleCI; blocks on policy violations
- **Dependency graph**: visualizes direct + transitive dependency relationships
- Deployment: SaaS, private cloud, or on-premises (air-gapped available)

**What makes FOSSA unique:**
- Binary scanning bridges the gap between source-based SCA and deployed artifacts
- SBOM lifecycle management: not just generation but ingestion, verification, and distribution
- Proprietary vulnerability prioritization reduces noise vs. raw CVSS

---

### SBOM Category Insights for a Living Architecture Blueprint

| Feature | CycloneDX | SPDX | Syft | FOSSA |
|---|---|---|---|---|
| Standard type | Specification | Specification | Tool | Platform |
| Primary focus | Security + Supply chain | License compliance | Generation | Full lifecycle |
| Services/APIs in BOM | ✓ | ✗ (packages only) | ✗ | ✗ |
| ML/AI support | ✓ (v1.5+) | ✓ (v3.0) | ✗ | ✗ |
| Hardware support | ✓ | ✗ | ✗ | ✗ |
| Binary scanning | ✗ | ✗ | Limited | ✓ |
| VEX support | ✓ | ✓ | ✗ | ✓ |
| Dependency graph | ✓ | ✓ | ✓ | ✓ |
| License list | ✗ | ✓ (canonical) | Via SPDX | ✓ |

**For a planner-style BOM:**
- CycloneDX's object model (components + services + dependencies + vulnerabilities + definitions) is the most transferable abstraction for a "living architecture BOM" — it explicitly models *both* library components *and* external service integrations
- The `compositions` completeness assertion is a unique pattern: explicitly marking what is known vs. unknown in the inventory
- `pedigree` and `provenance` tracking are powerful patterns for knowing *where* each component originated

---

## 3. Architecture Documentation Platforms

These tools go beyond ADRs to capture the full structural model of a system — components, containers, relationships, technology choices, and runtime behavior.

---

### 3.1 Structurizr (C4 Model)

**Source:** [structurizr.com](https://structurizr.com), [docs.structurizr.com](https://docs.structurizr.com)  
**Created by:** Simon Brown (also creator of the C4 model)  
**Type:** Commercial SaaS + free open-source Structurizr Lite (Docker)

**The C4 Model (foundation):**
Four levels of abstraction:
1. **System Context** — your system + external users and systems
2. **Container** — applications, databases, APIs within your system
3. **Component** — modules inside a container
4. **Code** — classes/entities (rarely diagrammed)

**Structurizr DSL — Key Capabilities:**
```
workspace "E-commerce Platform" {
    model {
        customer = person "Customer"
        orderService = softwareSystem "Order Service" {
            webapp = container "Web App" "React/JS"
            db = container "Database" "PostgreSQL" { tags "Database" }
        }
        customer -> orderService "Uses"
    }
    views {
        systemContext orderService "Context" { include *; autoLayout }
        container orderService "Containers" { include *; autoLayout }
    }
}
```

**Key Features:**
- **Single model, multiple views**: define elements once; render from any angle without duplication
- **Auto-layout**: eliminate manual diagram maintenance; Graphviz-based automatic positioning
- **`!include` directives**: split large models into team-owned files; assemble with `!include teams/payment-team.dsl`
- **`extends` keyword**: child workspaces inherit parent model elements — enables decentralized ownership
- **ADR integration**: `!adrs <path>` imports ADR files (supports adr-tools, MADR, Log4brains format)
- **Dynamic diagrams**: sequence diagrams showing interaction flows layered on the static model
- **Implied relationships**: if A uses B.sub, Structurizr infers A uses B — eliminates redundant declarations
- **Tags + themes**: CSS-like styling; tag elements to show/hide views
- **Workspace validation**: DSL parse errors prevent invalid relationships
- **Version control**: DSL files in Git; diagrams regenerate from source
- **Structurizr Lite**: free self-hostable Docker image for single-workspace use

**Technology Landscape Management:**
- Each container/component declares its technology stack inline: `container "API" "Node.js/Express"`
- Technology tags enable filtered views: "show me all AWS components"
- Cross-team composition via `extends` means each team's DSL declares their technology choices

**Living Document Experience:**
- CI/CD can auto-regenerate diagrams on DSL changes
- `autoLayout` means diagrams never become stale from manual repositioning
- The DSL *is* the source of truth; no separate "diagram file" vs. "data file"

**CRUD / Editing:**
- Create/Update: DSL text files in any editor; Structurizr UI for interactive editing in SaaS
- Read: rendered diagrams in browser (SaaS, Lite, or static export)
- Delete: remove from DSL

---

### 3.2 Archi / ArchiMate

**Source:** [archimatetool.com](https://www.archimatetool.com)  
**Standard:** ArchiMate 3.x (The Open Group — ISO/IEC 42010)  
**Type:** Free, open-source desktop application (Eclipse RCP, Java)

**ArchiMate Model Layers:**
- **Strategy**: Capabilities, Value Streams, Resources, Drivers, Assessments
- **Business**: Business Processes, Functions, Services, Roles, Actors, Collaboration
- **Application**: Application Components, Services, Functions, Interactions
- **Technology**: Nodes, Infrastructure Services, Artifacts, Communication Networks
- **Physical**: Physical Equipment, Facilities, Distribution Networks
- **Implementation & Migration**: Work Packages, Deliverables, Gap Analysis

**Key Features:**
- Full ArchiMate 3.x implementation — 56 element types with rigorous relationship rules
- **Viewpoints**: pre-defined and custom viewpoints (e.g., Application Landscape, Motivation, Migration)
- **Model reuse**: elements are shared across diagrams — change once, update everywhere
- **Relationships**: strict semantics (Association, Composition, Aggregation, Assignment, Realization, Serving, Access, Influence, Triggering, Flow, Specialization)
- **Custom properties**: user-defined attributes on any element
- **Export**: PDF, PNG, SVG, HTML (interactive), CSV, Open Exchange XML
- **Open Exchange Format**: interoperability with other EA tools
- **Collaboration plugin** (commercial): multi-user editing, version control
- **Reporting**: Jasper Reports templates for custom output formats

**Technology Landscape Management:**
- Technology layer maps infrastructure components
- Application layer maps software components and their services
- Cross-layer relationships show how business capabilities rely on applications which rely on infrastructure

**Strengths:**
- Most rigorous modeling language available (ArchiMate as a standard ensures semantic precision)
- Free and mature; widely used in enterprise architecture practices
- Excellent for formal EA governance and impact analysis

**Limitations:**
- Steep learning curve (ArchiMate spec is complex — 56 elements + relationship rules)
- Desktop-only (no native web/collaborative version)
- Diagram-centric rather than model-centric in practice
- No built-in ADR support; no technology lifecycle tracking

---

### 3.3 IcePanel

**Source:** [icepanel.io](https://icepanel.io)  
**Type:** Commercial SaaS  
**Founded:** 2022 (YC W23)

**Built on:** C4 model + interactive overlays + model-based diagramming

**Key Features:**
- **Model-based diagramming**: every element exists once in the model; drag onto any diagram — no duplication
- Automatically syncs changes across all diagrams when the model is updated
- **C4 levels**: Context → System → Container → Component hierarchy with fluid drill-down navigation
- **Areas**: group elements visually (e.g., "deployed together in Kubernetes cluster")
- **Tags**: multi-dimensional tagging (technology, release version, team, environment) — tags act as *selectors* to filter diagrams
- **Icons library**: technology icons for cloud services, databases, frameworks applied at container level
- **Flows** (diagram overlays): sequence-style overlays on top of C4 diagrams showing a use case's interaction sequence — no separate UML diagram needed
- **Trust score**: staleness indicator to prompt model updates
- **Versioning via freezing**: snapshot the model at a point in time
- **Link to code**: connect diagram elements to source control, wiki pages, or cloud resources — alerts if linked resource no longer exists
- **Orphan detection**: surfaces elements referenced in the model but not in any diagram
- **Smart layout**: adding/moving elements to areas auto-expands the container; intelligent positioning

**Living Document Experience:**
- Model-sync means one change propagates everywhere — no "which diagram is current?" problem
- Flows layer creates dynamic documentation without a separate tool
- Code links + stale alerts make the model self-maintaining
- Tags enable on-demand "release X view", "AWS components view", "team A owned services view"

**CRUD / Editing:**
- Full in-UI CRUD via drag-and-drop canvas
- Create elements: drag from palette
- Edit: click to open properties panel (name, description, tags, links)
- Delete: remove from model (removed from all diagrams automatically)
- Relationships: draw connections, name them, tag them, add descriptions

**Unique Features for a Planner Tool:**
- **Flows as overlays** — document dynamic behavior without creating new diagrams
- **Tag-as-filter** — a planner BOM can use tags for technology lifecycle stage, ownership, phase, risk level
- **Link to reality** — connecting model nodes to actual code/infrastructure creates a "live" BOM

---

### 3.4 LeanIX (SAP LeanIX)

**Source:** [leanix.net](https://www.leanix.net), [help.sap.com/docs/leanix/ea](https://help.sap.com/docs/leanix/ea/fact-sheets)  
**Type:** Commercial SaaS (acquired by SAP in 2023)  
**Focus:** Enterprise Architecture Management (EAM) at the portfolio level

**Core Concept: Fact Sheets**
A fact sheet is a single-page repository for an architectural object. There are **12 predefined fact sheet types**:

| Type | Description |
|---|---|
| Application | Software systems that process business data |
| Business Capability | What the business can do (technology-agnostic) |
| IT Component | Technical services, middleware, platforms (e.g., PostgreSQL, Kafka) |
| Data Object | Business data entities and data flows |
| Interface | Integration points between applications |
| User Group | Categories of users interacting with applications |
| Epic | Feature/project-level change initiatives |
| Project | IT transformation projects |
| Platform | Infrastructure/platform groupings |
| Provider | External vendors and suppliers |
| Technical Stack | Technology stack groupings |
| Domain | Business domains |

**Key Features:**
- **Meta model**: relationships between all 12 fact sheet types — maps "which applications support which business capabilities" and "which IT components support which applications"
- **Application Portfolio Management**: landscape views, functional/technical fit assessments, rationalization candidates
- **Lifecycle tracking per fact sheet**: Plan → Active → Phase Out → End of Life with roadmap planning
- **Surveys**: crowdsource data quality by sending surveys to application owners
- **Diagrams**: architecture diagrams embedded in fact sheets with relationships visualized
- **Roadmap planning**: target state architecture, transformation roadmaps, gap analysis
- **Data quality completion percentage** displayed on every fact sheet header
- **SAP Landscape Discovery** (2024): automated discovery of SAP applications from live SAP systems, populates fact sheets automatically
- **Integrations**: Jira, ServiceNow, Azure DevOps, GitHub, Confluence, PowerBI
- **API**: REST API for bulk updates

**Technology Landscape Management:**
LeanIX's strength is mapping the technology landscape at *portfolio scale*:
- Which 200 applications are approaching end-of-life?
- Which IT components (e.g., "Oracle Database 12c") are used by which applications?
- Which redundant applications support the same business capability?
- What is the data flow between Application A and Application B?

**CRUD / Editing:**
- Create: manual in UI, Excel import, API, or automated discovery
- Read: individual fact sheet pages, inventory table view, landscape diagrams
- Update: inline editing on fact sheet; bulk edit via table view; crowdsource via surveys
- Delete: archive fact sheets

**What makes UX effective:**
- Completion percentage creates data-quality accountability
- Surveys push data collection to the people who actually know (decentralized)
- The meta model enforces structural relationships — prevents orphan data

---

### Architecture Documentation — Cross-Tool Feature Comparison

| Feature | Structurizr | Archi | IcePanel | LeanIX |
|---|---|---|---|---|
| C4 model | ✓ (native) | ✗ | ✓ (native) | ✗ |
| ArchiMate | ✗ | ✓ (native) | ✗ | ✗ |
| Code-as-model (DSL) | ✓ | ✗ | ✗ | ✗ |
| In-UI editing | ✓ (SaaS) | ✓ | ✓ | ✓ |
| Auto-layout | ✓ | ✗ | ✓ | ✗ |
| Sequence/flow overlays | ✗ | ✗ | ✓ | ✗ |
| Tags/filtering | ✓ | ✓ | ✓ | ✓ |
| Technology lifecycle | ✗ | ✗ | Partial (trust score) | ✓ (full roadmap) |
| Portfolio-scale | ✗ | Limited | Limited | ✓ |
| ADR integration | ✓ | ✗ | ✗ | ✗ |
| Link to code | ✗ | ✗ | ✓ | ✗ |
| Stale-alert / live sync | ✗ | ✗ | ✓ | ✗ |
| Team collaboration | ✓ (SaaS) | Plugin | ✓ | ✓ |
| Free/open-source | Lite (free) | ✓ | ✗ | ✗ |

---

### Architecture Documentation Insights for a Living Architecture Blueprint

1. **Single-model, multiple-views** (Structurizr / IcePanel) is the critical pattern — define an element once; render it in any context without duplication or inconsistency
2. **Tags as first-class selectors** (IcePanel) enable on-demand "filtered BOM" views — show only components tagged "production", "deprecated", or "owned by team-alpha"
3. **Flows/overlays** (IcePanel) solve the "dynamic documentation" problem without multiplying diagrams
4. **Fact sheets with completion percentages** (LeanIX) drive data quality accountability — a planner tool should show each entry's completeness
5. **Lifecycle tracking** (LeanIX) is essential for a BOM: Plan → Active → Phase Out → End of Life per component

---

## 4. Technology Radar Tools

Technology Radars visualize the adoption lifecycle of technologies, techniques, platforms, and tools across an organization. Pioneered by Thoughtworks.

---

### 4.1 Thoughtworks Technology Radar + Build Your Own Radar (BYOR)

**Source:** [thoughtworks.com/radar](https://www.thoughtworks.com/radar), [radar.thoughtworks.com](https://radar.thoughtworks.com), [github.com/thoughtworks/build-your-own-radar](https://github.com/thoughtworks/build-your-own-radar)

**Core Model:**
- **Blips**: individual technology items (a framework, a tool, a technique, a platform)
- **Quadrants** (Thoughtworks standard): Languages & Frameworks | Tools | Techniques | Platforms
- **Rings** (Thoughtworks standard): Adopt → Trial → Assess → Hold
  - **Adopt**: safe default choice, high confidence, production-ready
  - **Trial**: seen to work; commit to exploring with a goal
  - **Assess**: worth understanding; worth building PoC
  - **Hold**: proceed with caution; not recommended for new projects

**Movement Indicators:**
- Triangle up: moved toward center (adoption increasing)
- Triangle down: moved toward edge (declining)
- Circle: no change
- Star: new entry

**Build Your Own Radar (BYOR) tool:**
- Input: public Google Sheet, private Google Sheet, or CSV/JSON file
- Columns: `name`, `ring`, `quadrant`, `isNew`, `description`
- Optional: `status` column (New / Moved In / Moved Out / No Change)
- Self-hosted version supports **custom ring and quadrant names** via environment variables: `RINGS=['Adopt','Trial','Assess','Hold']`
- Rendered as an interactive HTML5 canvas visualization
- License: AGPL-3.0

**CRUD / Editing:**
- Create/Update: edit the Google Sheet or CSV; re-generate the radar
- Read: interactive visualization at a URL
- No in-tool editing — the spreadsheet/file is the source of truth

**Key Features for Lifecycle Tracking:**
- Blip descriptions capture the *why* behind placement (1-2 paragraphs)
- Movement indicators show direction of travel over time
- "Fading": blips stable for two radars (one year) fade from the visualization to reduce clutter
- "Re-blipping": resurrect a faded blip with new commentary when circumstances change

**UX notes:**
- Interactive: click a blip to read its description
- Quadrant isolation: click a quadrant to zoom into it with sorted blip list
- The process of building the radar (the collaborative session) is as valuable as the output

---

### 4.2 Zalando Tech Radar

**Source:** [opensource.zalando.com/tech-radar](https://opensource.zalando.com/tech-radar), [github.com/zalando/tech-radar](https://github.com/zalando/tech-radar)

**Key Differentiators from Thoughtworks BYOR:**
- Open-source **d3.js-based visualization library** (not just a web app) — embed in any page
- **Tech Radar Compendium**: companion document with detailed summaries of each blip — description, use cases, risks, and which internal Zalando teams have used it
- **Principal Engineers** (Zalando's architecture board) curate the radar via ring change proposals and voting
- **Open contribution model**: any Zalando engineering team can submit new blips
- Configuration as JavaScript object: entries array with `{label, quadrant, ring, moved, link}` per blip
- `moved` values: -1 (moved out), 0 (no change), 1 (moved in), 2 (new)
- Custom ring semantics: ADOPT / TRIAL / ASSESS / HOLD (same as Thoughtworks)

**Maintenance Process:**
- Quarterly cadence
- Guild-based ownership: technology domain guilds curate their category
- REA Group variant: different rings (Adopt/Consult/Experiment/Hold/Retire) to reflect organizational nuance
- Automation: git issues trigger blip update workflows
- Curation frequency varies by ring: Adopt/Hold every 12 months; Trial/Experiment every 6 months

---

### 4.3 Tech Radar Key Patterns for a Planner Tool

**What a Tech Radar does well:**
1. **Lifecycle positioning** as a visual metaphor — the ring IS the lifecycle stage
2. **Movement as history** — a blip's ring changes over time tell the technology's adoption story
3. **Quadrant taxonomy** — organizing technologies into meaningful categories for navigation
4. **Mandatory commentary** (the write-up process): forces accountability; someone must own and justify each placement

**What Tech Radars lack:**
- No dependency relationships between blips (technology A depends on technology B)
- No ADR linkage (why did we move React from Trial to Adopt? The ADR lives elsewhere)
- No component/service mapping (which services *use* a given blip?)
- Static snapshots, not continuously updated

**Cross-Feature Comparison:**

| Feature | Thoughtworks BYOR | Zalando Tech Radar |
|---|---|---|
| Input source | Google Sheet / CSV | JavaScript array |
| Movement indicators | ✓ | ✓ |
| Custom quadrants/rings | ✓ (self-hosted) | ✓ |
| Companion compendium | ✗ | ✓ |
| Open-source library | ✓ (AGPL) | ✓ (MIT) |
| Fading mechanism | ✓ | ✗ (manual) |
| Contribution workflow | Manual | Guild-based |

**For a living architecture blueprint:**
- Tech Radar's quadrant + ring model is the ideal **technology lifecycle layer** of a BOM
- The key insight: every "technology" entry in the BOM should have a ring assignment (its organizational lifecycle stage), not just a version number
- The gap to fill: link radar blips to the components that use them, and to the ADRs that explain placement decisions

---

## 5. Developer Portals / Service Catalogs

Developer portals provide a unified interface for managing the full inventory of software components (services, libraries, APIs, pipelines, infrastructure) with rich metadata, ownership, dependencies, and quality tracking.

---

### 5.1 Backstage (Spotify)

**Source:** [backstage.io](https://backstage.io), [backstage.io/docs/features/software-catalog/](https://backstage.io/docs/features/software-catalog/)  
**Type:** Open-source framework. CNCF project. (Spotify-originated, now community-governed)

**Core Concept: catalog-info.yaml**
Every software entity is defined by a YAML descriptor file in its Git repository:
```yaml
apiVersion: backstage.io/v1alpha1
kind: Component
metadata:
  name: payment-service
  annotations:
    github.com/project-slug: org/payment-service
    backstage.io/adr-location: docs/adrs
spec:
  type: service
  lifecycle: production
  owner: payments-team
  dependsOn:
    - component:order-service
    - resource:postgres-payments-db
```

**Entity Kinds (out of the box):**
- `Component`: services, websites, libraries, ML models, data pipelines
- `API`: OpenAPI, AsyncAPI, gRPC, GraphQL specs
- `Resource`: databases, S3 buckets, cloud infrastructure
- `System`: a collection of related components
- `Domain`: a grouping of related systems
- `Group`: team or organization unit
- `User`: individual
- `Template`: Software Templates for scaffolding new components
- `Location`: pointer to other catalog descriptor files

**Key Features:**
- **Git-as-source-of-truth**: catalog harvests YAML from repos; updates via Git merge
- **Dependency graph**: `dependsOn` / `providesApis` / `consumesApis` relations; visual catalog graph in UI
- **Plugin ecosystem**: 200+ plugins integrating CI/CD, observability, security scanning, docs, cloud resources, PagerDuty, etc.
- **TechDocs**: docs-as-code (MkDocs-based) renders documentation inside Backstage from Markdown in the repo
- **Software Templates**: golden-path scaffolding; new services auto-register in catalog
- **Search**: full-text across catalog entities, TechDocs, ADRs
- **Lifecycle tracking**: `spec.lifecycle` field: experimental / production / deprecated
- **API catalog**: browse all APIs exposed by components with rendered specs

**CRUD / Editing:**
- Create: register via `/create`, auto-register via Templates, or static config
- Read: catalog browse at `/catalog`; entity detail pages with plugin tabs
- Update: edit YAML in Git → auto-propagated to Backstage
- Delete: unregister from catalog UI, or remove YAML from repo

**Dependency/Relationship Visualization:**
- Catalog graph view shows relationships between components, APIs, resources
- `dependsOn` creates directed dependency edges
- System/Domain hierarchy provides portfolio-level structure

**Strengths:**
- Massive plugin ecosystem turns Backstage into a unified developer portal
- Git-native metadata model — no separate database to maintain
- Scales from 10 to thousands of services (Spotify's production at thousands)

**Limitations:**
- Self-hosted only (CNCF project); operational overhead is significant
- No built-in quality scorecards (require Cortex-style plugins or custom implementation)
- Plugins require engineering investment to configure and maintain

---

### 5.2 Port

**Source:** [port.io](https://port.io), [docs.port.io](https://docs.port.io)  
**Type:** Commercial SaaS developer portal  
**Key differentiator:** Fully customizable data model via **Blueprints**

**Core Concepts:**

**Blueprint**: A schema defining a type of entity in your catalog
```json
{
  "identifier": "microservice",
  "title": "Microservice",
  "schema": {
    "properties": {
      "language": { "type": "string" },
      "tier": { "type": "string" },
      "team": { "type": "string" }
    }
  },
  "relations": {
    "database": { "target": "database", "required": false }
  }
}
```

**Entity**: An instance of a Blueprint (e.g., a specific microservice)

**Relation**: Typed link between blueprints — single (one entity to one target) or many (one entity to multiple targets)

**Key Features:**
- **General-purpose catalog**: model anything — microservices, CI/CD pipelines, K8s clusters, Terraform resources, environments, deployments, cloud resources, users, teams
- **Self-service actions**: forms + approval workflows connected to GitHub Actions, Azure Pipelines, Terraform, or any webhook — scaffold/deploy/provision/rollback from the portal
- **Scorecards**: quality gates with points-based or level-progression rules on any blueprint; tracks DORA metrics, security posture, production readiness, compliance
- **Dependency graph** (2025): visual graph of entity relations — see "cart service → kafka topic → order service → postgres" in one view
- **Integrations**: GitHub, GitLab, Jira, ArgoCD, Datadog, Dynatrace, Snyk, PagerDuty, K8s, AWS, Azure, GCP, Terraform, Pulumi
- **Users/Teams as blueprints** (2025): model ownership of any entity type, not just services
- **AI assistant**: natural language queries about the catalog; AWS blueprint/mapping suggestions
- **Surveys**: built-in survey system for collecting team feedback or tracking migrations
- **Customizable entity cards**: choose which fields and relations to surface on entity overview pages

**Dependency Visualization:**
- Dependency graph visualizes all entity relations in the catalog
- Filter by entity type, team, environment to focus the graph
- Answers: "which cloud resources in AWS us-east-1 affect these services during an outage?"

**CRUD / Editing:**
- Create: Port UI, API, Terraform provider, GitHub workflow, git-ops YAML (`port.yaml`)
- Read: catalog browse, entity detail pages, dependency graph
- Update: in-UI editing, API, Terraform, or automated integration sync
- Delete: UI or API

**Unique Features for a Planner Tool:**
- Blueprints make the data model itself a first-class editable configuration — define custom entity types for "Decision", "Pattern", "Constraint", "Quality Attribute"
- Relations create a typed knowledge graph across all entity types
- Self-service actions can trigger creation of new catalog entries automatically

---

### 5.3 Cortex

**Source:** [cortex.io](https://cortex.io), [docs.cortex.io](https://docs.cortex.io)  
**Type:** Commercial SaaS developer portal with emphasis on quality scorecards

**Key Features:**
- **Entity catalog**: services, resources, teams, users, custom entity types (via `cortex.yaml`)
- **Scorecards**: the core differentiator — define standards and automatically evaluate entities against them
  - Two scoring models: **level progression** (Bronze/Silver/Gold) or **point-based**
  - Rules via form builder or **CQL** (Cortex Query Language)
  - Built-in scorecard templates: Production Readiness, Security, DORA Metrics, AI Maturity, SOC 2 Compliance, Code Quality, Vulnerability Management, Secrets Management, Ownership Verification, JavaScript Best Practices, minimum version checks for AWS services
  - Evaluation every 4 hours (configurable); manual trigger available
  - **Initiatives**: time-boxed campaigns with deadline, owner assignment, progress tracking — migrate all services to TLS 1.3 by Q2
  - Rule exemptions: individual rules can be exempted with justification
  - Notifications when scores drop
  - **Leaderboards**: gamified team competition on scorecard scores
- **GitOps**: Scorecards-as-code via YAML
- **CQL (Cortex Query Language)**: query entity metadata across integrations to write complex rules
- **Integrations**: GitHub, GitLab, PagerDuty, Datadog, Splunk, SonarQube, Snyk, Jira, CircleCI, etc.

**CRUD / Editing:**
- Create entity: `cortex.yaml` in Git repo, or UI
- Create scorecard: UI wizard (from template or scratch) or YAML
- Read: catalog browse + scorecard dashboards + team leaderboards
- Update: YAML in Git or UI
- Delete: archive in UI

**Unique Features:**
- **CQL** enables rules that span multiple data sources: "does this service have PagerDuty on-call AND a passing SonarQube gate AND a recent Snyk scan?"
- **Initiatives** close the gap between "we measured a problem" and "we fixed it by a deadline" — actionable quality governance
- **AI Maturity scorecard template** reflects 2025 reality of AI-embedded engineering

---

### 5.4 OpsLevel

**Source:** [opslevel.com](https://opslevel.com), [docs.opslevel.com](https://docs.opslevel.com)  
**Type:** Commercial SaaS developer portal

**Key Features:**
- **Software catalog**: microservices, systems, teams, tools — unified inventory
- **Automated service detection**: identifies new services from connected Git, CI, and runtime tools; suggests additions with potential aliases — catalog stays current with low manual effort
- **Maturity Rubric**: centralized quality framework with maturity levels (Beginner → Bronze → Silver → Gold → custom)
  - Categories: Security, Quality, Reliability, etc.
  - Cross-cutting standards: consistent taxonomy ensures teams use same maturity language
- **Service-specific Scorecards**: per-service standards scoped to language, tier, or use case (e.g., Ruby Production Readiness)
- **Campaigns**: time-bound quality initiatives with deadlines — "migrate to Postgres 15 before EOL"; separate from permanent rubric checks; gives teams advance notice before check affects maturity score
- **Historical Reporting**: tracks maturity progress over time; trend charts (daily/weekly/monthly); shows which teams are improving/declining
- **Component Maturity Report**: organization-wide view; filter by team, tier, category; "Category Breakdown" shows precisely which category is dragging a service's overall level
- **Actions / Self-service**: templates for spinning up new services; operational task automation
- **MCP server integration** (2025): AI-powered queries about service ownership and runbooks directly in IDEs
- **Dependencies**: upstream/downstream impact mapping per service

**CRUD / Editing:**
- Create: automated detection + approval, `opal.yaml` in repo, GraphQL API, Terraform provider, K8s sinker
- Read: catalog browse, maturity report, historical charts, scorecard dashboards
- Update: YAML in Git, API, or UI
- Delete: archive

**Unique Features:**
- **Automated service detection** with alias resolution is the most sophisticated catalog-population mechanism of the four tools
- **Campaigns vs. Rubric separation**: the rubric is the permanent standard; campaigns are time-boxed on-ramps — prevents sudden maturity drops from new requirements

---

### Developer Portal — Cross-Tool Feature Comparison

| Feature | Backstage | Port | Cortex | OpsLevel |
|---|---|---|---|---|
| Open-source | ✓ | ✗ | ✗ | ✗ |
| Custom entity types | Via plugins | ✓ (blueprints) | ✓ | Limited |
| Self-service actions | ✓ (templates) | ✓ (full) | Limited | ✓ |
| Quality scorecards | Plugin required | ✓ | ✓ (core feature) | ✓ |
| ADR integration | ✓ (plugin) | ✗ | ✗ | ✗ |
| Dependency graph | ✓ | ✓ (2025) | ✓ | ✓ |
| Automated discovery | Limited | ✓ | ✓ | ✓ |
| Time-bound campaigns | ✗ | Via scorecards | ✓ (initiatives) | ✓ |
| Historical trend tracking | ✗ | Limited | ✓ | ✓ |
| Gamification | ✗ | ✗ | ✓ (leaderboards) | ✗ |
| GitOps (catalog-as-code) | ✓ | ✓ | ✓ | ✓ |
| MCP/AI integration | Limited | ✓ | ✓ | ✓ |
| On-prem deployment | ✓ | ✗ | ✗ | ✗ |

---

### Developer Portal Insights for a Living Architecture Blueprint

1. **Blueprints/entity types as the schema layer** (Port model): the catalog's data model should itself be configurable — enable custom entity types for "Decision", "Pattern", "Constraint", "Architecture Principle"
2. **Automated discovery** (OpsLevel): BOM entries should be auto-detected and proposed — not solely manually entered
3. **Scorecards + campaigns** (Cortex/OpsLevel): quality standards attached to BOM entries, with time-bound improvement paths
4. **Dependency graphs as knowledge maps**: the relationship between components is as important as the components themselves
5. **Historical tracking**: a "living BOM" needs trend data — which components improved, which degraded, and when

---

## 6. Cross-Category Analysis: Patterns for a Living Architecture Blueprint

### 6.1 Most Impactful Features Across All Categories

| Pattern | Source Tool(s) | Why It Matters |
|---|---|---|
| Single model, multiple views | Structurizr, IcePanel | Eliminate drift between diagram versions |
| Tags as selectors | IcePanel, LeanIX, Port | Filter BOM by any dimension (tech, team, lifecycle, risk) |
| Immutable records with supersession chains | All ADR tools | Full decision history; every "why" is preserved |
| Lifecycle rings | Tech Radar tools | Standardized organizational adoption stages for any technology |
| Contextual attachment | Backstage ADR plugin | Decisions live with the components they affect |
| Completion percentage | LeanIX | Drives data quality; surfaces gaps |
| Blueprint/entity type configurability | Port | The schema is configurable, not hardcoded |
| Automated discovery + approval | OpsLevel, Port | Catalog self-updates; reduced manual maintenance toil |
| Scorecards + campaigns | Cortex, OpsLevel | Measurable quality standards + time-bound improvement paths |
| `compositions` completeness | CycloneDX | Explicitly mark what is known vs. unknown in the inventory |
| Dependency graph | Port, Backstage, OpsLevel | Visualize second-order impacts of any change |
| Flows as overlays | IcePanel | Dynamic behavior without multiplying diagrams |

---

### 6.2 How Tools Handle CRUD/Editing

| Tool Category | Create | Read | Update | Delete |
|---|---|---|---|---|
| ADR tools | CLI wizard / PR | Static site / Backstage tab | Markdown edit in IDE | Not supported (immutability) |
| SBOM tools | CLI scan / CI pipeline | JSON/XML output file | Re-scan | Replace file |
| Architecture documentation | DSL/UI | Rendered diagrams | DSL edit / in-UI | Remove from DSL/UI |
| Tech Radar | Spreadsheet/CSV | Interactive HTML5 | Edit spreadsheet; regenerate | Remove from spreadsheet |
| Developer portals | YAML in git / UI / API | Catalog browse + entity pages | YAML in git / UI / API | Archive / unregister |

**Key insight**: All mature tools converge on **Git-as-source-of-truth** for the canonical record, with a read-optimized UI layer on top. The "YAML in git" model (Backstage, Cortex, OpsLevel) and "DSL in git" model (Structurizr) are the same principle applied to different domains.

---

### 6.3 How Tools Visualize Relationships and Dependencies

| Tool | Visualization Approach |
|---|---|
| adr-tools | DOT graph of ADR supersession relationships |
| Structurizr | C4 diagrams at 4 abstraction levels; implied relationships auto-drawn |
| IcePanel | Model-based C4 canvas; flows as sequence overlays; drill-down |
| LeanIX | Fact sheet relationship diagrams; application landscape matrix |
| ArchiMate/Archi | Layered viewpoints; strict relationship semantics |
| CycloneDX | Dependency graph in JSON/XML (`dependencies` element) |
| Backstage | Catalog graph with entity kind filtering |
| Port | Dependency graph (2025); entity relations via blueprint connections |
| Cortex | Service dependency maps; integration-data visualizations |
| OpsLevel | Upstream/downstream service impact maps |

---

### 6.4 How Tools Manage Knowledge Bases and Pattern Libraries

| Category | Knowledge/Pattern Mechanism |
|---|---|
| ADR tools | The ADR log itself IS the decision knowledge base. Patterns emerge from repeated decision contexts. |
| Tech Radar | Blip descriptions are mini-articles explaining why a technology occupies its ring. The Compendium (Zalando) is a curated pattern library. |
| Developer portals | TechDocs (Backstage) / wiki properties (Port) attach documentation to catalog entries. Templates encode "golden paths" as reusable patterns. |
| Architecture docs | Structurizr DSL `!docs` and `!adrs` attach documentation to model elements. LeanIX fact sheet descriptions carry structured knowledge. |
| SBOM tools | Not designed for knowledge management — focused on inventory and compliance. |

---

### 6.5 Unique Features Valuable in a Planner-Style Tool

| Feature | Source | Description |
|---|---|---|
| **Supersession chain** | All ADR tools | Every decision has a full lineage — proposed → accepted → superseded by → superseded by |
| **Ring lifecycle** | Tech Radar | Standardized 4-stage lifecycle (Assess / Trial / Adopt / Hold) applicable to any BOM entry, not just technologies |
| **Completeness assertions** | CycloneDX | `compositions` element explicitly marks what is known vs. unknown in the inventory |
| **`pedigree` / provenance** | CycloneDX | Track *where* a component decision originated — which team, which source |
| **Flows as overlays** | IcePanel | Document dynamic interaction behavior without creating separate diagrams |
| **Tags-as-selectors** | IcePanel | Enable on-demand filtered views of the BOM without maintaining separate views |
| **Blueprints (configurable schema)** | Port | The data model is configurable — add custom entity types for Patterns, Decisions, Constraints |
| **Campaigns** | OpsLevel, Cortex | Time-boxed improvement initiatives with deadlines — not just measuring current state but driving change |
| **Completion percentage** | LeanIX | Surface data quality gaps inline on every entry |
| **Automated discovery** | OpsLevel | Catalog entries suggested from existing tools — reduce the cold-start problem |
| **Link-to-reality** | IcePanel | Connect model nodes to actual code, cloud resources, wikis; alert on dead links |
| **Decision gate** | Arachne | Code cannot merge without an accepted ADR — decisions are enforceable prerequisites |
| **Services element** | CycloneDX | External API integrations are first-class BOM entries, not just package dependencies |
| **Historical trend tracking** | OpsLevel, Cortex | BOM quality scores over time; know if you're improving or regressing |
| **Scorecard templates** | Cortex | Reusable quality frameworks: Production Readiness, Security, DORA, AI Maturity |

---

### 6.6 Architectural BOM Abstraction Model (Synthesis)

Based on this research, a complete "living architecture blueprint" tool would need these layers:

```
┌─────────────────────────────────────────────────────────┐
│  DECISION LAYER         (ADR tools)                      │
│  Proposed → Accepted → Superseded chains                 │
│  Immutable records; cross-linked; contextually attached  │
├─────────────────────────────────────────────────────────┤
│  LIFECYCLE LAYER        (Tech Radar)                     │
│  Assess → Trial → Adopt → Hold per technology/pattern   │
│  Movement history; blip descriptions; quadrant taxonomy  │
├─────────────────────────────────────────────────────────┤
│  COMPONENT LAYER        (SBOM + Developer Portals)       │
│  Inventory of all components with full metadata          │
│  Dependencies: packages, services, APIs, infrastructure  │
│  Ownership, tier, language, version, purl, hashes        │
├─────────────────────────────────────────────────────────┤
│  STRUCTURE LAYER        (Architecture Docs)              │
│  C4-style model: Systems, Containers, Components         │
│  Relationships, data flows, technology declarations      │
│  Tags → filtered views; flows → dynamic overlays        │
├─────────────────────────────────────────────────────────┤
│  QUALITY LAYER          (Developer Portals)              │
│  Scorecards per entity: Production Readiness, Security   │
│  Campaigns: time-bound improvement initiatives           │
│  Completion %, historical trends, team leaderboards      │
├─────────────────────────────────────────────────────────┤
│  KNOWLEDGE LAYER        (Cross-cutting)                  │
│  Pattern library, principles, constraints                │
│  Linked to decisions that established them               │
│  Linked to components that implement them               │
└─────────────────────────────────────────────────────────┘
```

The most effective tools in each category succeed by:
1. **Making the source of truth a file in Git** — version controlled, diff-able, PR-reviewable
2. **Rendering a read-optimized UI layer** on top of those files — search, browse, visualize
3. **Cross-linking entries** — a decision links to the components it affects; a component links to its ADR; a lifecycle blip links to the services using it
4. **Automating discovery** — reducing the toil of keeping the inventory current
5. **Measuring completeness** — surfacing what is unknown, not just what is known

---

*Research compiled March 5, 2026. Sources: GitHub repositories, official documentation, engineering blogs from Zalando, Thoughtworks, Spotify/Backstage, Anchore, OWASP, Linux Foundation, SAP/LeanIX, IcePanel, Port, Cortex, OpsLevel.*
