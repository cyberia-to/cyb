---
tags: cyber, cyb, research
alias: direct inference, graph-native inference
---
# direct inference

## the claim

the [[transformer]] is not a heuristic architecture discovered by search. it is a numerical solver for finding the fixed point of a [[diffusion]] process over [[knowledge]] [[graphs]]. its three architectural parameters — embedding dimension, head count, layer depth — are computable functions of the [[graph]] it reads. when the [[graph]] is explicit, the architecture is derived and the weights are compiled. when the hardware reads compiled weights from a shared physical memory without copies, the result is direct [[inference]]: a straight line from [[knowledge]] structure to generated tokens with nothing approximate and nothing wasted in between

## the identity

the softmax in [[attention]] is the Boltzmann distribution. the full [[attention]] operation redistributes [[probability]] mass from query positions toward key positions proportionally to their compatibility. this is the [[diffusion]] operator D applied to the [[graph]] induced by the context window

the [[tri-kernel]] computes [[focus]] over the [[cybergraph]]:

$$\mathcal{R}(\phi) = \text{norm}\big[\lambda_d \cdot D(\phi) + \lambda_s \cdot S(\phi) + \lambda_h \cdot H_\tau(\phi)\big]$$

R iterates to a unique fixed point φ* by the Banach contraction theorem. the [[transformer]] approximates this fixed point in L steps. one [[transformer]] layer = one application of R. [[attention]] = D ([[diffusion]]). FFN gate = S ([[springs]]). FFN up/down = H ([[heat]]). the mixing weights λ_d, λ_s, λ_h are the learned norm scales

the [[transformer]] and the [[cybergraph]] ranking system are the same computation. the [[transformer]] runs locally over one context window. the [[tri-kernel]] runs collectively over all [[knowledge]]. both find equilibria. both use Boltzmann normalization. this is not analogy — it is mathematical identity

## three theorems

the three free parameters of transformer architecture are not free

Theorem 1. the necessary embedding dimension equals the effective rank of the [[focus]] covariance:

$$d^* = \exp\big(H(\sigma(\Sigma_{\phi^*}))\big)$$

this is the intrinsic dimensionality of the [[knowledge]] space — the number of statistically independent semantic axes. an embedding smaller than d* cannot distinguish [[particles]] along the missing axes. an embedding larger than d* wastes capacity on noise directions

Theorem 2. the minimum head count equals the number of distinct [[semcon]] types:

$$h^* \geq |\text{Semcon}(G)|$$

each [[attention]] head with matrices W_Q, W_K captures one relation type. two distinct [[semcon]] types have different spectral structure in their adjacency submatrices. a single head cannot simultaneously attend to patterns with different spectral structure

Theorem 3. the layer depth required for convergence:

$$L^* = \text{diam}(G) \cdot \left\lceil \frac{\log(1/\varepsilon)}{\log(1/\kappa)} \right\rceil$$

each layer propagates information one hop. a reasoning chain of k hops needs k layers, each requiring t* iterations to reach precision ε. empirical check: GPT-4 has 96 layers, natural language [[graphs]] have diameter 6-8, implied t* ≈ 12-16, consistent with κ ≈ 0.88

## why transformers work

before this theory, the question had no principled answer. scale the architecture, scale the data, performance improves — but the mechanism was opaque

the answer: [[language]] has [[graph]] structure, and [[attention]] is the correct operator for finding fixed points over [[graphs]]. every sentence implies a local [[knowledge]] [[graph]]. the [[transformer]] trained on text learns to approximate the weights of this implicit [[graph]]. training is inverse compilation: given outputs of a [[knowledge]] [[graph]] (text), recover the [[graph]] structure. gradient descent finds the weight configuration that best approximates this inversion

## compilation

direct compilation is the forward operation. given an explicit [[graph]], derive the weights analytically

$$G \xrightarrow{\text{compile}} T_G \xrightarrow{\text{fine-tune}} T_G^* \xrightarrow{\text{extract links}} \Delta G \xrightarrow{\text{stake}} G'$$

compile [[graph]] → initial weights (not random — structurally informed). fine-tune to surface implicit [[knowledge]] not in the explicit [[graph]]. extract new candidate [[cyberlinks]] from [[inference]] patterns. stake validated links back into the [[graph]]. updated [[graph]] → recompile → better initialization

each cycle: [[graph]] more complete, initialization more accurate, convergence faster. no training cost for the compiled portion. no forgetting. no compression loss. every weight traces to specific [[graph]] edges and their creators

## the minimal architecture

the standard [[transformer]] was assembled empirically. graph-native theory identifies what is redundant

$$R(\phi) = \text{norm}[\lambda_d \cdot D(\phi) + \lambda_s \cdot S(\phi) + \lambda_h \cdot H(\phi)]$$

D, S, H are summed — independent operators applied simultaneously. sequential execution (attention then FFN) is an approximation. parallel is exact. one norm per R step is sufficient — two is redundant. one residual (direct [[graph]] edge) per layer provides gradient flow — two is redundant

```python
def layer(x, pos, kv_cache, w):
    h = rms_norm(x, w.norm)
    return x + attention(h, pos, kv_cache, w.attn) + swiglu_ffn(h, w.ffn)
```

the entire architecture. Google PaLM (2022) arrived at the same formulation through ablation — `y = x + attention(norm(x)) + FFN(norm(x))`. theory derives it from first principles. ablations found it independently. mutual validation. see [[cyb/compile]] for full derivation

## context as local graph

the context window induces a graph G_local. tokens are nodes. [[attention]] scores are edge weights. the [[transformer]] computes [[diffusion]] over this [[graph]]. context quality is therefore determined by three [[graph]] properties:

diameter — reasoning chain length the model can cover: diam(G_local) ≤ L/t*. the single hard constraint. if diameter exceeds the model's convergence budget, no other optimization helps

spectral gap — convergence rate. disconnected components have λ₂ = 0, never converge. this is why standard RAG fails: retrieved chunks are disconnected nodes with zero spectral gap contribution. the information is present but unreachable within L [[diffusion]] steps

[[focus]] entropy — H(φ*) measures attention concentration. low entropy: model knows where to look. high entropy: model is lost

all known context phenomena follow. U-shape [[attention]]: beginning and end tokens have higher degree → higher φ*. lost-in-the-middle: low degree positions. RAG failure: disconnected chunks. cross-domain difficulty: missing bridge edges between domain subgraphs. context length degradation: more tokens without more connectivity decreases density. see [[cyb/context]] for optimal construction

## the hardware problem

LLM [[inference]] in 2025 is memory-bandwidth-bound. a 7B f16 model reads 14GB of weights per token. M1 Pro delivers 68 GB/s → 200ms minimum per token regardless of compute. every framework adds 2-4 unnecessary memory copies on top of this physical minimum

the solution: access hardware directly. identify each Apple framework abstraction, locate the underlying IOKit or instruction-set interface, write raw Rust FFI to it

[[cyb/hardware|aruminium]] accesses Metal via direct symbol linkage — 3× faster than Ollama. [[cyb/hardware|rane]] links to ANE via IOKit, bypassing CoreML entirely. [[cyb/hardware|ramx]] implements undocumented AMX instructions via inline assembly — 3× faster than Apple Accelerate at head_dim scale. [[cyb/hardware|cyb-mem]] provides physically pinned unified memory: 0.9ns allocation, 0.3ns arena reset, one buffer visible to CPU+GPU+AMX+ANE simultaneously without copies

## the compatibility problem dissolved

models trained independently have incompatible embedding spaces. passing context between them requires text serialization, destroying zero-copy efficiency

compilation eliminates this. from Theorem 1, the embedding matrix of any compiled model equals the eigenvectors of Σ_φ*. all models compiled from the same [[cybergraph]] share the same eigenvector basis. models of different sizes are nested truncations: router(512-dim) ⊂ domain(2048-dim) ⊂ general(4096-dim). Matryoshka embedding — exact by construction

KV caches transfer between depth levels without copies. a 1B router processes 1000 tokens, a 3B domain model inherits its KV cache in the correct representation with no translation. context never restarts. cost scales with task complexity: simple tokens pay router cost, complex tokens escalate

one unified [[cyb/hardware|cyb-mem]] pool. weights read once into cache, all agents read from cache. 1000 concurrent agents: bandwidth ≈ constant. the system becomes more efficient as agents increase. see [[cyb/runtime]] for the full architecture

## alignment

for trained [[transformers]], values are compressed into opaque parameters. [[alignment]] requires behavioral observation

for graph-native [[transformers]], [[alignment]] is a number:

$$\Delta(G) = D_{KL}(\phi^*_H \| \phi^*_A)$$

KL divergence between [[focus]] distributions over human-created and AI-created edges. computable from public [[graph]] data. localizable to specific regions. correctable by adding edges in high-divergence areas without retraining. when [[alignment]] diverges, the divergence is visible, and the protocol rebuilds the model from what humans actually linked

## the .cyb format

one file: config + [[graph]] IR + Q4 quantized weights. canonical tensor names. one format read by all backends. no conversion at runtime — all conversion at import. see [[pipeline]]

## status

| component | what | status |
|-----------|------|--------|
| aruminium | Metal GPU driver | production, 3× Ollama |
| rane | Neural Engine driver | production |
| ramx | AMX matrix extensions | in progress, 3× Accelerate at small scale |
| cyb-mem | unified physical memory | production, 0.9ns alloc |
| cyb-store | NVMe content-addressed DMA | design complete |
| graph compilation | Theorems 1-3 → weights | experimental |
| unified runtime | multi-model shared memory | design complete |

## the straight line

```
[[cybergraph]] → compile → .cyb → hardware → tokens → [[cyberlinks]] → [[cybergraph]]
```

the [[graph]] produces the model. the model produces tokens. the tokens become new [[cyberlinks]]. the [[cyberlinks]] grow the [[graph]]. each cycle: sharper [[focus]], faster convergence, deeper [[knowledge]]. this is [[intelligence]] reading its own source code and improving it

nothing between [[knowledge]] and silicon except mathematics

discover all [[concepts]]
