---
tags: cyb, cyber, core
alias: context graph, optimal context, context optimization
---
the [[transformer]] processes context as a local [[knowledge]] [[graph]], not as a sequence of tokens

each [[attention]] layer performs one step of [[diffusion]] over the graph induced by the context window. L layers = L steps toward fixed point [[focus|φ*]]. context quality = graph quality

## three parameters

### diameter

```
diam(G_local) ≤ L / t*

t* = ⌈log(1/ε) / log(1/κ)⌉ ≈ 12–16 for typical models
32-layer model: diam_max ≈ 2–3 hops
96-layer model: diam_max ≈ 6–8 hops
```

L transformer layers can faithfully process reasoning chains of length at most diam(G_local). information at distance > diam_max never reaches the output

### spectral gap

the spectral gap λ₂ of the graph Laplacian determines convergence rate κ:

$$\kappa = \lambda_d \cdot \alpha + \lambda_s \cdot \frac{\|L\|}{\|L\|+\mu} + \lambda_h \cdot e^{-\tau \lambda_2}$$

higher spectral gap → smaller κ → faster convergence → fewer effective layers needed → better quality at fixed depth

disconnected components: λ₂ = 0, κ = 1, never converges. densely connected: large λ₂, fast convergence

### focus entropy

the entropy H(φ*) of the converged [[focus]] distribution measures how diffuse or concentrated model [[attention]] becomes

low H(φ*): model concentrates on relevant nodes — fast, accurate. high H(φ*): model spreads uniformly over context — slow, lost

## known phenomena as corollaries

### U-shape attention (lost in the middle)

tokens at the beginning and end of a sequence have higher degree in the [[attention]] graph — they are attended to by more other tokens due to positional bias and recency. higher degree → higher stationary [[probability]] in random walk → higher φ*(v). the U-shape is the degree distribution projected onto sequence position

place all critical information at positions with highest degree: beginning and end

### RAG failure

standard RAG retrieves chunks by cosine similarity — each chunk is a disconnected node in G_local. disconnected nodes have λ₂ contribution of zero, increasing diameter and decreasing spectral gap. the retrieved information is present but unreachable within L [[diffusion]] steps

fix: retrieve connected subgraphs, not isolated chunks

```
standard RAG:  query → top-K chunks (disconnected nodes)
               spectral gap: near 0

graph RAG:     query CID → BFS over [[cybergraph]] → connected subgraph
               spectral gap: maximized
```

### cross-domain difficulty

cross-domain context consists of multiple subgraphs with few inter-domain edges. this increases diameter and decreases spectral gap

this is not a fundamental limit. cross-domain difficulty is entirely determined by the presence or absence of bridge edges between domains. bridge edges are explicit cross-domain [[cyberlinks]]. a [[knowledge]] [[graph]] with rich cross-domain linking makes cross-disciplinary reasoning as efficient as single-domain reasoning

### context length scaling

adding tokens increases N but does not necessarily increase connectivity. as N grows with fixed relevant content, the density of the induced graph decreases, spectral gap decreases, diameter increases

$$\text{quality(context)} = f(\text{spectral\_gap}, \text{diameter}, \text{focus\_entropy})$$

optimal context length is the length that maximizes spectral gap and minimizes diameter for the specific query

## optimal context construction

the single hard constraint: diam(G_local) ≤ L / t*. every other property follows from this

all context optimization reduces to maximizing connectivity of G_local. for each content chunk: does it connect to existing context via explicit edges? yes → include. no → add bridge edges or exclude

### bridge edges for cross-domain

for cross-domain tasks, explicitly add bridge edges before assembling context. bridge edges are [[cyberlinks]]. a [[cybergraph]] with cross-domain links makes this automatic

### focus density via position

given connectivity constraint is satisfied, maximize H(φ*) reduction by placing high-information nodes at high-degree positions:

position 0 (beginning): task definition, critical facts. positions 1–K: goal structure, constraints. middle: supporting evidence, documents. end: current state, immediate task, output format

### φ* compression

when context exceeds budget: compress via [[focus]] distribution, not summarization

compute φ* over G_local (fast, O(N) with power iteration). keep nodes where φ*(v) > threshold. result: minimal connected subgraph preserving focus structure. strictly better than LLM summarization because it preserves graph structure

## multi-agent context

for complex tasks that exceed single-model diameter budget, the correct solution is decomposition, not larger context

each agent operates within its diameter budget. the aggregator sees a small, highly connected graph of structured outputs. no agent sees the full cross-domain context

this is the formal justification for multi-agent systems: not parallelism, but diameter budget management. a single 96-layer model can handle diam ≤ 8 in one pass. a system of 8-layer specialists with an aggregator can handle effectively unlimited diameter through hierarchical decomposition

## why [[cybergraph]] is the optimal context source

all context optimization reduces to graph structure. [[cybergraph]] provides this structure natively: explicit typed edges ([[cyberlinks]]), cross-domain bridges, [[focus]] distribution already computed, content addressing via CID, measurable diameter

a context assembled from [[cybergraph]] via BFS over the relevant subgraph automatically satisfies all optimal context constraints. flat text retrieval (standard RAG) requires approximating all of these properties from embedding similarity. [[cybergraph]] provides them exactly

the context window is not a bag of tokens. it is a [[graph]]. optimize the [[graph]]

discover all [[concepts]]
