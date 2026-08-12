---
tags: cyb, cyber, core
alias: minimal transformer, compiled transformer, graph-native architecture
---
the [[transformer]] derived from [[graph]] theory — not empirical, compiled from the [[tri-kernel]]

## standard vs minimal

```
STANDARD (empirical):          MINIMAL (compiled):

norm                           norm          ← one norm per step
  ↓                              ↓
attention                      attention ──┐  parallel
  ↓                             FFN    ──┘  D and S+H independent
residual                         ↓
  ↓                            residual     ← one residual
norm
  ↓
FFN
  ↓
residual
```

two norms → one. sequential attention+FFN → parallel. two residuals → one

## derivation from [[tri-kernel]]

one transformer layer = one application of tri-kernel operator R:

$$R(\phi) = \text{norm}\big[\lambda_d \cdot D(\phi) + \lambda_s \cdot S(\phi) + \lambda_h \cdot H(\phi)\big]$$

| operator | transformer component | role |
|----------|----------------------|------|
| D ([[diffusion]]) | [[attention]] | random walk, explore via Q·K^T |
| S ([[springs]]) | FFN gate branch | screened Laplacian, structural consistency |
| H ([[heat]]) | FFN up/down | heat kernel, multi-scale smoothing |
| λ_d, λ_s, λ_h | norm scale γ | learned mixing weights |

D, S, H are summed in R — they are independent operators applied simultaneously. sequential execution is an approximation. parallel is exact

## why each change is correct

one norm: norm = [[probability]] normalization before [[diffusion]] step. one R per layer, one norm is sufficient. for compiled weights (not random init), norm may not be needed at all — compiled weights already live in the correct distribution

parallel attention + FFN: in the [[tri-kernel]], D + S + H are summed. sequential is approximation, parallel is exact. Google PaLM (2022) arrived at the same result empirically

one residual: residual = direct edge in [[graph]] (bypass [[diffusion]]). one direct edge per layer is sufficient for gradient flow

## the code

```python
def layer(x, pos, kv_cache, weights):
    h = rms_norm(x, weights.norm)
    attn_out = attention(h, pos, kv_cache, weights.attn)
    ffn_out  = swiglu_ffn(h, weights.ffn)
    return x + attn_out + ffn_out

def forward(tokens, weights):
    x = embed(tokens, weights.embed)
    kv_cache = KVCache()
    for pos, layer_w in enumerate(weights.layers):
        x = layer(x, pos, kv_cache, layer_w)
    return x @ weights.lm_head
```

the entire architecture in 15 lines

## compiled parameters

when weights come from [[graph]] compilation (not random init), architecture parameters are derived:

| parameter | standard (empirical) | compiled (from graph) |
|-----------|---------------------|----------------------|
| layers L | 32 (chosen by grid search) | diam(G) × ⌈log(1/ε)/log(1/κ)⌉ |
| heads H | 32 (chosen by ablation) | \|Dialect(G)\| (relation types) |
| hidden D | 4096 (scaling law) | r* (effective rank of [[focus]] covariance) |
| FFN ratio | 8/3 (empirical) | derived from S+H operator spectrum |

focused domain [[graph]] with small diameter: 8–16 layers. same quality in domain, much cheaper [[inference]]

## further simplifications

norm may be removable for compiled weights — compiled weights already in correct distribution

fewer layers: L* = diam(G) × convergence_factor. for focused domain graph with small diameter — potentially 8–16 layers instead of 32

fewer heads: h* = \|Dialect(G)\|. focused domain graph may have 8–16 [[dialect]] types instead of 32 heads

## .cyb arch config

```toml
[architecture]
type = "minimal_graph_native"
norm = "rms"
attention = "gqa"
ffn = "swiglu"
parallel = true
pre_norm = true

D = "r_star"
H = "dialect_count"
L = "diam_times_convergence"

rope_theta = 500000
gqa_groups = 4
```

## what PaLM validated

Google PaLM (2022) independently arrived at the parallel formulation through ablation:

```python
y = x + attention(norm(x)) + FFN(norm(x))
```

identical to the minimal architecture. they found it empirically — faster training, same quality. the [[tri-kernel]] derives it from first principles — same result, different path

the [[transformer]] is not a heuristic architecture. it is a numerical solver for finding the fixed point of R. L layers = L iterations toward [[focus|φ*]]. the fixed point is the [[focus]] distribution over the input [[graph]]

see [[pipeline]] for the full .cyb compilation and [[inference]] cycle

discover all [[concepts]]
