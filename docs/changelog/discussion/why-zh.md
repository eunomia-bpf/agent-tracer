# AI智能体可观测性

## 问题/差距

### 1. **"AI智能体发展迅速且不同于传统软件"**

AI驱动的智能体系统的兴起正在改变现代软件基础设施。像AutoGen、LangChain、Claude Code和gemini-cli这样的框架编排大型语言模型（LLM）来自动化软件工程任务、数据分析管道和多智能体决策。与产生确定性、易于观察行为的传统软件组件不同，这些AI智能体系统生成开放式、非确定性的输出，通常以隐藏的内部状态和多个智能体之间的涌现交互为条件。因此，调试和监控智能体软件带来了前所未有的可观测性挑战，这是经典的应用性能监控（APM）工具无法充分解决的。

### AI智能体可观测性与经典软件可观测性的区别

| 维度 | 传统应用/微服务 | LLM或多智能体系统 |
| --- | --- | --- |
| **你试图"看到"什么** | 延迟、错误、CPU、GC、SQL计数、请求路径 | *语义* — 提示词/工具追踪、推理步骤、毒性、幻觉率、人格偏移、花费的令牌/金钱 |
| **基本事实** | 确定性规范：给定X必须产生Y或异常 | 开放式输出：许多"可接受的"Y；质量通过相似性、有用性或政策合规性来判断 |
| **故障模式** | 崩溃、5xx、内存泄漏、死锁 | 错误事实、无限推理循环、遗忘指令、智能体之间的涌现式错误协调 |
| **时间尺度** | 毫秒级跨度；状态通常在请求结束时消失 | 对话历史和暂存记忆可以存活数小时或数天；"状态"隐藏在向量数据库行和系统提示中 |
| **信号源** | 你有意发出的结构化日志和指标 | 通常*在纯文本TLS有效载荷内*；以及工具执行日志 |
| **修复工作流程** | 重现、附加调试器、修补代码 | 重新提示、微调、更改工具连接、调整护栏——代码可能没问题，但"思考过程"是错误的 |
| **安全/审计** | 追踪显示运行了什么代码 | 需要证据说明模型*为什么*说了某些话，用于合规/事件审查 |

为什么这种差异对研究很重要？

**仪表化差距** – 智能体逻辑和算法每天都在变化（新提示、工具）或在运行时自行更改。依赖代码内钩子意味着持续的变动；内核侧或sidecar追踪保持稳定。

**语义遥测** – 我们需要新的跨度属性（"model.temp"、"tool.role"、"reasoning.loop_id"）和新的异常检测器（矛盾、人格偏移）。

**因果融合** – 研究挑战：将低级事件与高级语义跨度合并到单一时间线中，以便SRE可以回答"为什么我的代码不工作？它在什么系统上运行，你尝试了什么命令？"

**防篡改** – 如果提示注入使智能体变成恶意的，它可能会消除自己的日志。进程外和内核级追踪提供了独立的审计通道。

简而言之，AI智能体可观测性继承了AI智能体的**不可靠、涌现行为**。将智能体运行时视为半信任的黑盒，并在系统边界进行观察：这就是机会所在。

### 2. **"当前的可观测性技术依赖于应用级仪表化"**

当前的智能体可观测性技术主要依赖于应用级仪表化——回调、中间件钩子或显式日志记录——集成在每个智能体框架内。虽然直观，但这种方法存在三个基本限制。首先，智能体框架发展迅速，经常更改提示、工具、工作流和内存接口。它们甚至可以修改自己的代码来创建新工具、更改提示和行为。因此，嵌入在智能体代码库中的仪表化会产生大量的维护开销。其次，智能体运行时可能被篡改或妥协（例如，通过提示注入），允许攻击者或错误行为完全逃避日志记录。第四，应用级仪表化无法可靠地捕获跨智能体语义，如推理循环、语义矛盾、人格转变，或者它与环境交互时的行为，特别是当交互跨越进程或二进制边界时（例如，外部工具或子进程）。

对于安全性，考虑一个LLM智能体首先编写带有恶意命令的bash文件（不执行，安全），然后使用基本工具调用执行它（通常允许）。它需要系统范围的可观测性和约束。

## AI智能体可观测性格局

以下是截至2025年7月LLM/AI智能体可观测性工具的快速格局扫描。我专注于（a）暴露SDK、代理或规范，你可以今天就连接到智能体堆栈中，以及（b）提供某种方式来追踪/评估/监控生产中的模型调用的产品。

| #  | 工具/SDK（首次发布年份） | 集成路径 | 它能提供什么 | 许可证/模式 | 备注 |
| -- | --------------------------------------------------- | ------------------------------------------------------------------ | ------------------------------------------------------------------------------------------ | ------------------------------ | ------------------------------------------------------------------------------------------------------------- |
| 1  | **LangSmith** (2023) | 在任何LangChain/LangGraph应用中添加`import langsmith` | 请求/响应追踪、提示词和令牌统计、内置评估作业 | SaaS，免费层 | 与LangChain集成最紧密；OTel导出处于beta阶段。([LangSmith][1]) |
| 2  | **Helicone** (2023) | 即插即用的反向代理或Python/JS SDK | 记录每个OpenAI风格的HTTP调用；实时成本和延迟仪表板；"智能"模型路由 | 开源核心(MIT) + 托管 | 代理模型保持应用代码不变。([Helicone.ai][2], [Helicone.ai][3]) |
| 3  | **Traceloop** (2024) | 一行AI-SDK导入 → OTel | 提示词、工具、子调用的完整OTel跨度；重放和A/B测试流程 | SaaS，慷慨的免费层 | 使用标准OTel数据；适用于任何后端。([AI SDK][4], [traceloop.com][5]) |
| 4  | **Arize Phoenix** (2024) | `pip install arize-phoenix`；OpenInference追踪器 | 本地UI + 向量存储用于追踪；使用另一个LLM进行自动评估（毒性、相关性） | Apache-2.0，自托管或云 | 提供自己的开源UI；适合离线调试。([Phoenix][6], [GitHub][7]) |
| 5  | **Langfuse** (2024) | Langfuse SDK *或* 发送原始OTel OTLP | 嵌套追踪、成本指标、提示管理、评估；在Docker中自托管 | 开源(MIT) + 云 | 在RAG/多智能体项目中很受欢迎；OTLP端点保持供应商中立。([Langfuse][8], [Langfuse][9]) |
| 6  | **WhyLabs LangKit** (2023) | 提取文本指标的包装器 | 漂移、毒性、情感、PII标志；发送到WhyLabs平台 | Apache-2.0核心，付费云 | 添加重量级文本质量指标而非请求追踪。([WhyLabs][10], [docs.whylabs.ai][11]) |
| 7  | **PromptLayer** (2022) | 装饰器/上下文管理器或代理 | 提示链的时间线视图；差异和重放；基于OTel跨度构建 | SaaS | 早期推动者；最小代码更改但非开源。([PromptLayer][12], [PromptLayer][13]) |
| 8  | **Literal AI** (2024) | Python SDK + UI | RAG感知追踪、评估实验、数据集 | 开源核心 + SaaS | 面向发布聊天机器人的产品团队。([literalai.com][14], [literalai.com][15]) |
| 9  | **W\&B Weave / Traces** (2024) | `import weave` 或 W\&B SDK | 深度链接到现有W\&B项目；捕获代码、输入、输出、用户反馈 | SaaS | 如果你已经使用W\&B进行ML实验，这很好。([Weights & Biases][16]) |
| 10 | **Honeycomb Gen‑AI views** (2024) | 发送OTel跨度；Honeycomb UI | 提示跨度、延迟、错误的热图 + BubbleUp | SaaS | 建立在Honeycomb成熟的追踪存储之上；没有评估层。([Honeycomb][17]) |
| 11 | **OpenTelemetry GenAI semantic‑conventions** (2024) | 规范 + contrib Python库（`opentelemetry-instrumentation-openai`） | 模型、智能体、提示的标准跨度/指标名称 | Apache-2.0 | 提供通用语言；上述几个工具都发出它。([OpenTelemetry][18]) |
| 12 | **OpenInference spec** (2023) | 追踪器包装器（支持LangChain、LlamaIndex、Autogen…） | 追踪的JSON模式 + 插件；Phoenix使用它 | Apache-2.0 | 规范，而非托管服务；与任何OTel后端配合良好。([GitHub][19]) |

### 格局告诉我们什么

* **几乎每个人都在SDK层挂钩。** 12个选项中有11个要求你包装或代理函数调用。这对概念验证来说很好，但当智能体热交换提示或生成绕过包装器的新工具时就会失效。
* **OpenTelemetry正在成为事实上的线路格式。** Traceloop、Honeycomb、Langfuse、PromptLayer、Phoenix（通过OpenInference）都使用OTel，这简化了后端选择。
* **语义评估仍处于早期阶段。** 只有Phoenix、LangSmith、Langfuse和Literal提供内置的LLM驱动的质量检查（毒性、相关性、幻觉评分）。大多数其他工具专注于延迟 + 成本。
* **没有人进行内核级捕获。** 列出的工具都不直接观察加密的TLS缓冲区或`execve()`调用；它们相信应用层是诚实的。这为提示注入或自修改智能体留下了盲点——这正是零仪表化eBPF追踪器可以填补的差距。
* **规范 vs. 平台。** OpenTelemetry GenAI和OpenInference降低了集成税，但不存储或可视化任何东西；你仍然需要后端。相反，SaaS平台捆绑存储、查询和评估，但将你锁定在它们的数据形式中。

### 这如何激发"边界追踪"的想法

因为今天的解决方案*大多*存在于智能体进程内部，它们继承了与智能体代码相同的脆弱性：

* **当你调整提示图时会中断** – 每个新节点都需要一个装饰器。
* **被恶意提示逃避** – 受损的智能体可以丢弃或伪造日志。
* **对跨进程副作用视而不见** – 例如，编写shell脚本然后`execve()`执行它。

一个系统级的eBPF追踪器，它收集TLS写缓冲区和系统调用，可以避开这些问题：

| 今天的SDK停止的地方 | 边界追踪仍然能看到的 |
| -------------------------------------------------- | ------------------------------------- |
| 当智能体直接生成`curl`时缺少跨度 | `execve("curl", …)` + 网络写入 |
| 智能体在记录之前修改自己的提示字符串 | 离开TLS套接字的原始密文 |
| 子进程误用GPU | `ioctl` + CUDA驱动程序调用 |

换句话说，现有工具解决了"我的代码内部发生了什么？"的故事；内核侧追踪可以回答"实际上什么击中了网络和操作系统？"——一个互补的、更难篡改的优势点。

这个差距为研究和开源创新敞开了大门。

## **关键见解和观察**

所有现有AI智能体系统的有意义交互都有两个清晰的穿越边界：

> AI智能体可观测性必须与智能体内部解耦。**从边界观察提供了稳定的语义接口**。
>

### AI智能体结构

智能体中心堆栈作为三个嵌套圆圈：

```text
┌───────────────────────────────────────────────┐
│          ☁  工作空间/系统的其余部分              │
│  (API、数据库、消息总线、操作系统、Kubernetes…)    │
│                                               │
│   ┌───────────────────────────────────────┐   │
│   │       智能体运行时/框架                   │   │
│   │ (LangChain、claude-code、gemini-cli …)│   │
│   │  • 编排提示和工具调用                     │   │
│   │  • 拥有暂存内存/向量数据库                 │   │
│   └───────────────────────────────────────┘   │
│               ↑ 出站API调用                    │
│───────────────────────────────────────────────│
│               ↓ 入站事件                       │
│   ┌───────────────────────────────────────┐   │
│   │          LLM服务提供商                   │   │
│   │    (OpenAI端点、本地llama.cpp)          │   │
│   └───────────────────────────────────────┘   │
└───────────────────────────────────────────────┘
```

* **LLM服务提供商** – 令牌生成、非确定性推理、可能会或可能不会浮出水面的思维链文本。大多数系统工作都围绕LLM服务层。
* **智能体运行时层** – 将任务转换为LLM调用序列加上外部工具调用；存储临时"记忆"。
* **外部世界** – 操作系统、容器、其他服务。

对于**可观测性目的**，清晰的接口通常是*网络边界*（JSON推理请求的TLS写入）和系统边界（智能体命中命令`curl`、`grep`时的系统调用/子进程）。这些线以下的任何东西（GPU内核、权重矩阵、模型）都是模型推理服务领域；以上的任何东西都是经典的系统可观测性任务。这就是为什么内核级eBPF可以给你一个中立的优势：它跨越两个世界而不需要库钩子。

传统软件可观测性是**仪表化优先**（你在编写的代码中插入日志、跨度和指标）。

但AI智能体通过提示、指令、推理路径和自发的工具使用动态地改变其内部逻辑。这种持续的内部可变性意味着*仪表化是脆弱的*。

通过将可观测性转移到稳定的**系统级边界**——内核系统调用接口、TLS缓冲区、网络套接字——你可以实现：

* **框架中立性**：跨所有智能体运行时工作（LangChain、AutoGen、gemini-cli）。
* **语义稳定性**：捕获提示级语义而不追逐框架API。
* **信任和可审计性**：不能轻易被智能体内恶意软件妥协的独立追踪。
* **通用因果图**：将智能体级语义与操作系统级事件合并到一个连贯的故事中。

---

## 系统构建

1. 一个完全基于**系统级追踪（eBPF）**构建的AI智能体系统零仪表化可观测性工具，以实现独立于快速发展的智能体运行时和框架的统一语义和操作可见性。
2. 一个LLM"sidecar"方法来检测细微的语义异常（例如，推理循环、矛盾、人格转变）以及系统日志。

## 挑战

核心挑战在于内核级信号和AI智能体行为之间的**语义差距**。虽然eBPF可以以最小的开销（通常是2-3%的CPU使用率）捕获全面的系统级数据，但将其转化为关于智能体性能的有意义的见解需要复杂的关联技术。

另一个挑战是从加密的TLS流量中捕获所有提示和与后端服务器的交互。大多数LLM服务使用TLS与后端服务器通信，并使用SSE流式传输响应。使用传统的网络数据包捕获工具如tcpdump或wireshark是不够的，因为流量是加密的。代理流量可以是一个替代解决方案，但代理解决方案需要显式的配置更改来通过代理路由智能体流量，这可能不适用于第三方应用程序或框架，并且可能引入额外的延迟和复杂性。即使现有的eBPF工具可以捕获流量，它也缺乏对SSE流API的支持。

通过使用eBPF uprobe在用户空间挂钩TLS读写，我们可以捕获流量并解密它。

## 参考文献

[1]: https://docs.smith.langchain.com/observability?utm_source=chatgpt.com "Observability Quick Start - ️🛠️ LangSmith - LangChain"
[2]: https://www.helicone.ai/?utm_source=chatgpt.com "Helicone / LLM-Observability for Developers"
[3]: https://www.helicone.ai/blog/llm-observability?utm_source=chatgpt.com "LLM Observability: 5 Essential Pillars for Production ... - Helicone"
[4]: https://ai-sdk.dev/providers/observability/traceloop?utm_source=chatgpt.com "Traceloop - Observability Integrations - AI SDK"
[5]: https://www.traceloop.com/?utm_source=chatgpt.com "Traceloop - LLM Reliability Platform"
[6]: https://phoenix.arize.com/?utm_source=chatgpt.com "Home - Phoenix - Arize AI"
[7]: https://github.com/Arize-ai/phoenix?utm_source=chatgpt.com "Arize-ai/phoenix: AI Observability & Evaluation - GitHub"
[8]: https://langfuse.com/?utm_source=chatgpt.com "Langfuse"
[9]: https://langfuse.com/docs/tracing?utm_source=chatgpt.com "LLM Observability & Application Tracing (open source) - Langfuse"
[10]: https://whylabs.ai/langkit?utm_source=chatgpt.com "LangKit: Open source tool for monitoring large language models ..."
[11]: https://docs.whylabs.ai/docs/large-language-model-monitoring/?utm_source=chatgpt.com "Large Language Model (LLM) Monitoring | WhyLabs Documentation"
[12]: https://docs.promptlayer.com/running-requests/traces?utm_source=chatgpt.com "Traces - PromptLayer"
[13]: https://www.promptlayer.com/platform/observability?utm_source=chatgpt.com "Complete AI Observability Monitor and Trace your LLMs - PromptLayer"
[14]: https://www.literalai.com/?utm_source=chatgpt.com "Literal AI - RAG LLM observability and evaluation platform"
[15]: https://www.literalai.com/open-source?utm_source=chatgpt.com "Test, Monitor and Improve LLM apps - Literal AI"
[16]: https://wandb.ai/site/traces/?utm_source=chatgpt.com "Enterprise-Level LLMOps: W&B Traces - Wandb"
[17]: https://www.honeycomb.io/ai-llm-observability?utm_source=chatgpt.com "Observability for AI & LLMs - Honeycomb"
[18]: https://opentelemetry.io/docs/specs/semconv/gen-ai/?utm_source=chatgpt.com "Semantic conventions for generative AI systems | OpenTelemetry"
[19]: https://github.com/Arize-ai/openinference?utm_source=chatgpt.com "Arize-ai/openinference: OpenTelemetry Instrumentation for ... - GitHub"