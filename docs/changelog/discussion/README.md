# helpful materials

https://arxiv.org/html/2507.12472v1 A Survey of AIOps in the Era of Large Language Models

We have the same definition as them:
We first define an agent as an interactive entity that uses an LLM as the core for reasoning, decision-making, and reflection while integrating memory, tools, and the environment as capability-enhancing components.
In system domain, it can be 1. one process (mainly, like claude code or gemini-cli) 2. multiple process 3. embed in other applications/process

## challenges



A key challenge is:
How can we collect the data at a reasonable overhead,
How can we effectively analyze the log.
As I said before, LLM agents are different from traditional software, it can evolve and change itself arbitrarily, it can create software or even create other agents and interact with it, it can be one process or multiple processes, or embedded into some software. It can be written in any language and can have arbitrary behavior. It's hard to determine "what needs to be traced" and "how to analyze the results" the traditional way, like allow humans to explore the data, write filters or specify what they want to capture.
Instead of relying on human experts, my idea is: can we use llm agents to observe and manage/detect agents behavior system widely? It specifies what it wants to know about the system and ai agent through programmable filtering or probing, and then analyse the logs.

I need more data and experiments to demonstrate that, maybe (we can use well established benchmarks for evaluation.) 

## Design


yusheng
  7 月 15 日，下午 5:22
Is the TLS traffic all that useful? It is not immediately obvious how knowing all of the HTTP requests is that useful.
Industry people and OSS community already has some practice around AI Agent observability (With eBPF or without eBPF). Capture the TLS traffic is the first step, because the agent’s input/output to the LLM server includes all the prompts, thinking, tool calls, and a lot of related AI Agent internals. The prompts and "How they use LLMs" are the key stuff that we need to observe and understand.
7 条回复


Andi Quinn
  7 月 15 日，晚上 7:20
My intuition is that safety isn’t a property of how the agent uses the LLM, but rather a property of how it interacts with the system. This is because it seems like the damage/danger only occurs through system interactions not through LLM ones.
:+1:
1



yusheng
  7 月 15 日，晚上 7:59
Yes, that's true.
8:02
My point is that observing these prompts and TLS traffic are like analyzing the call graph or control flow of traditional software, with static or dynamic analyzer. It's about how the llm agent works and how it thinks, not just how it interacts with the environment. We need observability from both directions. （已编辑） 


Andi Quinn
  7 月 15 日，晚上 10:22
You’re assuming that safety is analyzable with context. That is, knowing the prompts would increase a tool’s ability to analyze the system’s safety.


yusheng
  7 月 15 日，晚上 10:33
Yes. This has been proved by previous AI research, they are mainly analyzing AI agent's safety through prompts and llm generated answers/tool call recordings.
Our claim is more like "knowing the prompt is not enough", knowing the system behavior can increase a tool's ability to analyze the system's safety. （已编辑） 


yusheng
The AI side of this work can be similar to and based on https://arxiv.org/abs/2406.09187 (ICML 2025).
We can adopt a similar approach and evaluation plan, just to focus more on the system side. （已编辑） 

GuardAgent: Safeguard LLM Agents by a Guard Agent via Knowledge-Enabled Reasoning
The rapid advancement of large language model (LLM) agents has raised new concerns regarding their safety and security. In this paper, we propose GuardAgent, the first guardrail agent to protect target agents by dynamically checking whether their actions satisfy given safety guard requests. Specifically, GuardAgent first analyzes the safety guard requests to generate a task plan, and then maps this plan into guardrail code for execution. By performing the code execution, GuardAgent can deterministically follow the safety guard request and safeguard target agents. In both steps, an LLM is utilized as the reasoning component, supplemented by in-context demonstrations retrieved from a memory mo… 

I think we need to do something updated, at least similar to what the AI community is using and exploring.

## Evaluation

For example, you can see hundreds of papers from https://arxiv.org/html/2504.15585v4#S6 in LLM(-Agent) Deployment Safety section (This is a recent survey covering 900 paper and gets 25+ cites). LLM Agents are mainly in 6.2 to 6.5 

There are also lots of benchmarks, we can just use them for evaluate our claims