<p align="center">
  <img src="docs/assets/logo.png" alt="Open Ontologies" width="300">
</p>

<h1 align="center">Open Ontologies</h1>

<p align="center">
  <strong>规划对生产本体的变更，在应用之前看清每一项后果，<br>
  并交给审阅者一份无需信任你即可自行核验的证明。</strong><br>
  使用 Rust 编写，以单一可执行文件发布。
</p>

<p align="center">
  <a href="https://github.com/fabio-rovai/open-ontologies/stargazers"><img src="https://img.shields.io/github/stars/fabio-rovai/open-ontologies?style=for-the-badge&logo=github" alt="Stars"></a>
  <a href="https://github.com/fabio-rovai/open-ontologies/network/members"><img src="https://img.shields.io/github/forks/fabio-rovai/open-ontologies?style=for-the-badge&logo=github" alt="Forks"></a>
  <a href="https://github.com/fabio-rovai/open-ontologies/actions/workflows/ci.yml"><img src="https://img.shields.io/github/actions/workflow/status/fabio-rovai/open-ontologies/ci.yml?branch=main&style=for-the-badge" alt="CI"></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/License-MIT-blue.svg?style=for-the-badge" alt="MIT"></a>
  <a href="https://github.com/fabio-rovai/obsidian-open-ontologies"><img src="https://img.shields.io/badge/Obsidian-plugin-7C3AED?style=for-the-badge&logo=obsidian&logoColor=white" alt="Obsidian plugin"></a>
  <a href="https://github.com/sponsors/fabio-rovai"><img src="https://img.shields.io/github/sponsors/fabio-rovai?style=for-the-badge&label=Sponsor&logo=GitHub%20Sponsors&logoColor=EA4AAA&color=EA4AAA" alt="Sponsor"></a>
</p>

<p align="center">
  <a href="README.md">English</a> · <strong>简体中文</strong>
</p>

---

> 本文是英文 [README.md](README.md) 的中文版。英文版为准：当两者出现差异时，请以英文版为最新内容。

Open Ontologies 是一个 **Rust 编写的 MCP 服务器**与**桌面版 Studio**，面向 AI 原生的本体工程。它提供 **109 个工具**，让 Claude 能够基于内存中的 Oxigraph 三元组存储来构建、校验、查询、比对、检查、版本化、推理、对齐、规划、认证和治理 RDF/OWL 本体，并具备完整的 Dynamics → Causal → Planner 三层架构、33 个标准本体的市场、临床术语交叉映射、语义向量以及完整的血缘审计链路。语义向量、Postgres 与 DuckDB 连接器、WASM 插件宿主都是编译期可选特性。预编译二进制与 GHCR 镜像均以默认特性集构建，不包含上述功能；如需启用，请从源码执行 `cargo build --release --features embeddings,plugins,sql`。默认构建会公开全部 109 个工具，其中 8 个需要启用可选的 Cargo feature 才能运行：4 个需要 `embeddings`，2 个需要 `plugins`，2 个需要 `postgres` 或 `duckdb`；未启用时这些工具会返回错误。

**Studio** 将引擎封装为可视化桌面环境：带层级连线的虚拟化本体树、面包屑导航与关系浏览器；支持 `/build`（IES 级深度建模）与 `/sketch`（快速原型）指令的 AI 对话面板；Protégé 风格的属性检查器；以及血缘查看器。

无需 JVM，无需 Protégé。

---

<p align="center">
  <img src="https://raw.githubusercontent.com/fabio-rovai/open-ontologies/main/docs/assets/knowledge-graph.svg" alt="ies-core.ttl 的 Studio 三维视图：类与子类边在三维空间中布局，验证层作为节点绘制在旁边。" width="100%">
</p>

<p align="center">
  <sub><b>426 条断言，259 条已认证，1 条被拒绝。</b>绿色边是引擎推导出、并由 Lean 4 检查器<i>证明</i>过的。
  红色边是一条伪造的推导，同一个检查器拒绝了它：退出码 1，并指明了规则。
  所有计数都取自实际运行，而不是写死在说明文字里。</sub>
</p>

<p align="center">
  <img src="https://raw.githubusercontent.com/fabio-rovai/open-ontologies/main/docs/assets/hqdm-audit.svg" alt="HQDM 0.0.1-alpha 绘制为一个包含 490 个术语的连通图。23 个被当作类使用却从未声明的术语画作空心红环；12 条以关系而非类作为 rdfs:range 的声明画作汇聚到两个枢纽的红边。" width="100%">
</p>

<p align="center">
  <sub><b>同一套机制用在别人的文件上，结果并不干净。</b>
  <code>hqdmTop/hqdmFramework</code> 发布的 HQDM（MagmaCore 逐字节原样收录）不含任何
  <code>owl:</code> 术语，也没有任何不相交公理，因此<i>没有任何具名类可能不可满足</i>，
  一致性检查必然返回零。但它仍然有 <b>23</b> 个被当作类使用却从未声明的术语、
  <b>12</b> 条把关系而不是类写成 <code>rdfs:range</code> 的声明，以及
  <b>13</b> 对仅差一个尾部下划线的名称，其中 3 对的定义域与值域完全相同。
  这里没有任何东西被证明过，图上也没有这样声称；所有计数都由测试从原始三元组重新算出。
  来源与方法见 <a href="docs/assets/hqdm/PROVENANCE.md"><code>docs/assets/hqdm/PROVENANCE.md</code></a>。</sub>
</p>


---

## 核心能力

| 层 | 内容 |
|---|---|
| **Dynamics（动态层）** | `ActionSchema` 与 4 个 MCP 工具：`onto_action_register` / `_applicable` / `_apply` / `_list`。支持并发原子时刻、静态因果律（不变式）、默认值规则、基于 OWL-RL 闭包的连带效应，以及可复现随机种子的非确定性结果。 |
| **Causal（因果层）** | `onto_certify_action`，可选启用 PyWhy 后门识别（通过 `causal-pywhy` 特性开启）。默认使用结构代理，可选启用 do-演算，并具备优雅降级。 |
| **Planner（规划层）** | `onto_plan_compile_pddl` + `onto_plan_classical`（Fast Downward 子进程）+ `onto_plan_validate`（沙箱模拟）。求解器保留在客户端，服务端负责编译与校验。 |

设计约定：**服务端只提供校验与脚手架，智能部分由通过 MCP 连接的大模型完成。** 服务端内部不含任何 LLM 客户端，不需要 API 密钥，也没有供应商抽象层。

---

## 快速开始

### 安装

**预编译二进制：**

```bash
# macOS（Apple Silicon）
curl -LO https://github.com/fabio-rovai/open-ontologies/releases/latest/download/open-ontologies-aarch64-apple-darwin
chmod +x open-ontologies-aarch64-apple-darwin && mv open-ontologies-aarch64-apple-darwin /usr/local/bin/open-ontologies

# Linux（x86_64）
curl -LO https://github.com/fabio-rovai/open-ontologies/releases/latest/download/open-ontologies-x86_64-unknown-linux-gnu
chmod +x open-ontologies-x86_64-unknown-linux-gnu && mv open-ontologies-x86_64-unknown-linux-gnu /usr/local/bin/open-ontologies
```

**Docker：**

```bash
docker pull ghcr.io/fabio-rovai/open-ontologies:latest
docker run -i ghcr.io/fabio-rovai/open-ontologies serve
```

> `serve` 启动的是**通过标准输入输出进行 JSON-RPC 通信的 MCP 服务器**，并非交互式命令行。因此启动后它会"卡住"等待 MCP 客户端连接，这是预期行为。若想直接在终端试用，请使用 CLI 子命令（例如 `open-ontologies validate <file.ttl>`）。

**从源码构建（需 Rust 1.85+）：**

```bash
git clone https://github.com/fabio-rovai/open-ontologies.git
cd open-ontologies && cargo build --release --features embeddings,plugins,sql
./target/release/open-ontologies init
```

### 连接 MCP 客户端

<details>
<summary><strong>Claude Code</strong></summary>

在 `~/.claude/settings.json` 中加入：

```json
{
  "mcpServers": {
    "open-ontologies": {
      "command": "/path/to/open-ontologies/target/release/open-ontologies",
      "args": ["serve"]
    }
  }
}
```

重启 Claude Code 后，`onto_*` 系列工具即可使用。
</details>

<details>
<summary><strong>Claude Desktop</strong></summary>

在 `~/Library/Application Support/Claude/claude_desktop_config.json` 中加入同样的配置。
</details>

<details>
<summary><strong>Obsidian</strong></summary>

[Obsidian 版 Open Ontologies 插件](https://github.com/fabio-rovai/obsidian-open-ontologies)会把本引擎作为托管子进程运行在 Obsidian 内部：本体树、SPARQL 控制台、校验面板、Turtle 文件保存即校验，以及"仓库转 RDF"映射器——让你的笔记变成推理机可以处理的图谱。插件还会在一个固定且需要鉴权的回环端口上暴露 MCP 接口，因此 Claude Code 或 Claude Desktop 可以直接查询经过推理的仓库图谱。

详见 [docs/obsidian.md](docs/obsidian.md)。仅支持桌面端。
</details>

---

## 典型工作流

### 生成 → 校验 → 推理 → 验证

1. 直接生成 Turtle/OWL（Claude 原生掌握 OWL、RDF、BORO 与四维建模）
2. 调用 `onto_validate` 校验语法，失败则修正后重试
3. 调用 `onto_load` 载入 Oxigraph 三元组存储
4. 调用 `onto_stats` 确认类、属性与三元组数量符合预期
5. 调用 `onto_reason`（`rdfs` 或 `owl-rl` 配置）物化推理出的三元组
6. 调用 `onto_lint` 检查缺失的标签、注释、定义域与值域
7. 调用 `onto_enforce` 检查设计模式合规性
8. 调用 `onto_query` 用 SPARQL 验证结构并回答能力问题
9. 调用 `onto_save` 持久化，再调用 `onto_version` 保存快照以便回滚

关键原则：**Claude 根据上一个工具的返回值动态决定下一步调用。** MCP 工具是一个个独立操作，编排者是 Claude。

### 生产环境的本体演进

```
onto_plan（评估影响面与风险）
  → onto_enforce（设计模式检查）
  → onto_apply（safe 或 migrate 模式）
  → onto_monitor（SPARQL 监视器与阈值告警）
  → onto_drift（版本比对、重命名识别与自校准置信度）
```

---

## 其他发布渠道

同一引擎还以 [Docker 镜像](https://github.com/fabio-rovai/open-ontologies/pkgs/container/open-ontologies)、[PyPI 包](https://pypi.org/project/open-ontologies-lite/)与 [Obsidian 插件](https://github.com/fabio-rovai/obsidian-open-ontologies)的形式发布。

## 文档

完整的工具清单、基准测试、IES 支持说明、架构设计与案例研究，请参见英文 [README.md](README.md) 及 [docs/](docs/) 目录。

## 许可证

MIT。详见 [LICENSE](LICENSE)。
