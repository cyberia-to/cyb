---
tags: cyber, core
crystal-type: pattern
crystal-domain: cyber
crystal-size: deep
alias:: deterministic 3d rendering, deterministic rendering, cyberworld rendering
---

# deterministic 3d rendering

The [[tri-kernel]] converges to a unique [[focus]] distribution for any given [[cybergraph]] state. This uniqueness extends to spatial layout: one graph, one set of parameters, one world. Every [[neuron]] running the same protocol on the same graph sees a topologically identical three-dimensional structure, with cluster membership and BVH geometry bit-identical, positions ε-identical, and pixel output perceptually indistinguishable.

The rendering pipeline has three stages: spectral geometry from [[springs]], scale hierarchy from [[heat]], and flow dynamics from [[diffusion]]. Together they produce a complete, deterministic, navigable 3d world from raw graph topology.

The formal specification is [[render]] (R-1.0). The T∞ tier uses the [[compiled transformers spec]] CT-1.1 model as its neural backbone, making render cost per-pixel rather than per-particle — the same frame budget holds from 10⁶ to 10¹⁵ particles. [[clifford]] defines the multivector primitive extensions that make the wedge product and shifted geometric product first-class operations across cybergraph, CT-1.1, and render T∞. The primary backend is [[honeycrisp]] (`aruminium` for Metal raster, `rane` for ANE inference, `acpu` for AMX eigensolve, `unimem` for zero-copy IOSurface).

## spectral geometry

The [[springs]] operator is a screened [[Laplacian]]:

$$(L + \mu I)x^* = \mu x_0$$

The Laplacian $L = D - A$ encodes the full topology of the [[cybergraph]]. Its eigenvectors provide canonical coordinates for every [[particle]] in euclidean space. The first three nontrivial eigenvectors of $L$ become the $(x, y, z)$ position of each node.

This is spectral embedding. Unlike force-directed layout, which depends on initialization and converges to local minima, spectral coordinates are determined entirely by the graph structure. The eigenvectors are unique up to sign and rotation, both of which are fixed by convention:

- sign: the first nonzero component of each eigenvector is positive
- rotation: the [[crystal]] (5,040 irreducible particles) serves as the coordinate frame, anchoring orientation to genesis

The screening parameter $\mu$ controls the effective range of structural forces. High $\mu$ localizes coordinates around reference positions $x_0$, producing tight clusters. Low $\mu$ lets the global topology dominate, spreading nodes across the full spectral manifold.

The Green's function $(L + \mu I)^{-1}$ decays exponentially with graph distance. Distant nodes exert negligible influence on each other's positions. This means layout is h-local: editing a neighborhood only repositions nodes within $O(\log(1/\varepsilon))$ hops.

## scale hierarchy

The [[heat]] kernel provides intrinsic multi-scale structure:

$$H_\tau = \exp(-\tau L)$$

The temperature parameter $\tau$ acts as a continuous zoom level. At small $\tau$, only immediate neighbors interact — the world shows fine-grained local structure. At large $\tau$, the kernel smooths across entire communities, revealing the macro-architecture of [[crystal-domain]] clusters and thematic regions.

This gives the 3d world a natural level-of-detail system:

| $\tau$ | scale | what is visible |
|---|---|---|
| 0.01 | atomic | individual particles and their direct links |
| 0.1 | enzyme | local neighborhoods, small motifs |
| 1.0 | bridge | cross-domain connections, thematic corridors |
| 10.0 | article | domain-level clusters as coherent regions |
| 100.0 | deep | the global shape of the entire knowledge structure |

The semigroup property $H_{\tau_1} H_{\tau_2} = H_{\tau_1 + \tau_2}$ means scales compose cleanly. Zooming from local to global is continuous and reversible. No information is created or destroyed — the heat kernel only redistributes existing focus across scale.

Practically, $\tau$ maps to camera distance. As a viewer moves closer, the rendering evaluates the heat kernel at smaller $\tau$, resolving finer structure. As the viewer pulls back, larger $\tau$ aggregates particles into domain clusters, each rendered as a composite region with [[crystal-domain]] coloring.

Chebyshev polynomial approximation keeps computation local: a $K$-term expansion achieves $O(K)$-hop locality with bounded error, meaning each scale level requires only local graph traversal.

## flow dynamics

The [[diffusion]] operator animates the world:

$$\pi^{(t+1)} = \alpha P^\top \pi^{(t)} + (1 - \alpha) u$$

Where $P$ is the token-weighted transition matrix and $\alpha$ is the teleport parameter. This produces a probability flow across every [[cyberlink]] — a current of [[focus]] flowing through the graph.

In the 3d world, diffusion becomes visible motion:

- edges carry directional flow proportional to transition probabilities
- nodes pulse with accumulated focus (the [[cyberank]] score $\phi_i^*$)
- teleport events appear as ambient luminosity from the prior distribution $u$

The stationary distribution $\pi^*$ determines the brightness landscape. High-[[cyberank]] particles glow as attractors; low-rank particles exist as dim ambient structure. The flow itself traces paths of attention through the world — streams of probability that a random-walking [[neuron]] would follow.

The teleport parameter $\alpha$ controls the balance between following links (exploitation) and jumping to random locations (exploration). Low $\alpha$ produces concentrated flow along high-traffic corridors. High $\alpha$ distributes luminosity more evenly across the world.

## crystal properties as visual encoding

The [[crystal]] metadata system maps directly to rendering properties. Every particle carries typed metadata that determines its visual representation:

### shape from crystal-type

Nine [[crystal-type]] values correspond to nine geometric primitives:

| crystal-type | geometry | rationale |
|---|---|---|
| entity | sphere | complete, self-contained, fundamental |
| pattern | torus | cyclic, repeating structure |
| process | helix | temporal, sequential unfolding |
| property | tetrahedron | minimal polyhedron, attributive |
| measure | cylinder | scaled, oriented, quantitative |
| relation | edge bundle | connective, bridging |
| reference | pointer / arrow | directive, indexical |
| article | scroll / plane | flat, readable surface |
| observed | lens / eye | empirical, perceptual |

### color from crystal-domain

Seventeen [[crystal-domain]] values map to a fixed palette. Domains that are conceptually adjacent share similar hues:

- cyber, cybics, cyberia: blue spectrum (protocol core)
- mathematics, physics, computer science: violet spectrum (formal sciences)
- biology, chemistry, agriculture: green spectrum (life sciences)
- economics, governance, history: amber spectrum (social sciences)
- culture, superhuman: red spectrum (emergent phenomena)
- materials, energy, geography: earth tones (physical infrastructure)

### scale from crystal-size

Five [[crystal-size]] levels determine the physical radius of each node in the 3d world:

| crystal-size | relative radius | role |
|---|---|---|
| atom | 1x | fundamental concept, irreducible |
| enzyme | 2x | focused contribution, single operation |
| bridge | 3x | interdisciplinary connector |
| article | 4x | medium-depth explanation |
| deep | 6x | comprehensive specification |

### luminosity from focus

The [[focus]] value $\phi_i^*$ at each particle sets its emissive intensity. Focus is conserved:

$$\sum_{i=1}^n \phi_i(t) = 1 \quad \forall t$$

This conservation law means the total light in the world is constant. Focus flowing to one region dims another. The brightness landscape shifts as the graph evolves, but total luminosity is invariant.

## determinism proof

The rendering is deterministic because every stage maps a unique input to a unique output:

### stage 1: graph state is content-addressed

Every [[particle]] is identified by its content hash (64-byte Hemera hash). Every [[cyberlink]] is a signed triple (source, predicate, target) with a deterministic weight from [[token]] balances. The graph state is a Merkle-committed data structure — any two nodes with the same root hash have identical graph content.

### stage 2: tri-kernel has a unique fixed point

The composite operator $\mathcal{R} = \lambda_d D + \lambda_s S + \lambda_h H_\tau$ is a contraction with coefficient:

$$\kappa = \lambda_d \alpha + \lambda_s \frac{\|L\|}{\|L\| + \mu} + \lambda_h e^{-\tau \lambda_2} < 1$$

By the Banach fixed-point theorem, there exists exactly one $\phi^*$ such that $\mathcal{R}(\phi^*) = \phi^*$. The protocol fixes the weights $\lambda_d, \lambda_s, \lambda_h$ and parameters $\alpha, \mu, \tau$.

### stage 3: spectral coordinates are canonical

The eigenvectors of $L$ are determined by the graph structure. Sign convention and rotation anchoring to the [[crystal]] frame eliminate the remaining degrees of freedom. The eigendecomposition of a symmetric matrix is unique when eigenvalues are distinct; for repeated eigenvalues, the crystal anchor resolves the eigenspace ambiguity.

### stage 4: visual encoding is a pure function

Crystal metadata (type, domain, size) maps to (shape, color, radius) through a fixed lookup table. Focus maps to luminosity through a fixed transfer function. No randomness enters at any stage.

### composition

$$\text{graph state} \xrightarrow{\text{Laplacian eigenvectors}} (x,y,z) \xrightarrow{\text{crystal metadata}} (\text{shape, color, radius}) \xrightarrow{\text{focus } \phi^*} \text{luminosity} \xrightarrow{\text{heat } \tau} \text{LOD}$$

Each arrow is a deterministic function. The composition is deterministic. One graph produces one world.

## free energy as world physics

The tri-kernel fixed point minimizes a free-energy functional:

$$\mathcal{F}(\phi) = \lambda_s \left[\frac{1}{2}\phi^\top L\phi + \frac{\mu}{2}\|\phi - x_0\|^2\right] + \lambda_h \left[\frac{1}{2}\|\phi - H_\tau \phi\|^2\right] + \lambda_d \cdot D_{KL}(\phi \| D\phi)$$

This functional governs the physics of the rendered world:

- the elastic term $\frac{1}{2}\phi^\top L\phi$ penalizes focus discontinuities across edges — it is the discrete analog of gravitational potential energy, pulling connected nodes into coherent focus levels
- the screening term $\frac{\mu}{2}\|\phi - x_0\|^2$ anchors focus to reference positions, preventing unbounded drift
- the heat alignment term $\frac{1}{2}\|\phi - H_\tau \phi\|^2$ penalizes deviation from the smoothed context, enforcing scale-consistent rendering
- the KL divergence $D_{KL}(\phi \| D\phi)$ aligns the focus distribution with its own diffusion image, ensuring flow consistency

The world sits at the minimum of this functional. Perturbations (new cyberlinks, changed token weights) shift the minimum, and the tri-kernel iteration rolls downhill to the new equilibrium. The viewer sees the world relax into its new configuration — a physical process with well-defined dynamics.

## incremental rendering

The [[tri-kernel]] is h-local: an edit at any node requires recomputation only within $h = O(\log(1/\varepsilon))$ hops. This locality extends to rendering:

- spectral coordinates shift only in the neighborhood of the edit (perturbation theory of eigenvalues)
- focus redistribution is bounded by the contraction coefficient $\kappa$
- heat kernel updates propagate outward at a rate controlled by $\tau$

A viewer watching the world in real time sees local deformations: new particles fade in, existing ones drift to accommodate, focus flows redistribute. The global structure remains stable because distant eigenvalues are insensitive to local perturbations.

Light clients verify rendering correctness by checking:
- the Merkle commitment of the graph state
- boundary flow constraints at the edge of the recomputed neighborhood
- the focus residual $\|\mathcal{R}(\phi) - \phi\|$ is below threshold

Verification overhead is constant-factor relative to computation.

## the world is the graph

The key claim is not that we can render the cybergraph as a 3d world. The claim is that the cybergraph already is a world — the [[tri-kernel]] is its physics, [[focus]] is its conserved energy, and the [[crystal]] metadata are its material properties. Rendering does not impose structure from outside. It reveals structure that is intrinsic to the graph.

The Laplacian $L = D - A$ is the discrete analog of the Laplace-Beltrami operator $\nabla^2$ on continuous manifolds. The springs equation $(L + \mu I)x^* = \mu x_0$ is the discrete analog of the screened Poisson equation $(\nabla^2 + \mu)\Phi = \mu\Phi_0$, which governs gravitational potential with massive screening. The heat equation $\partial H / \partial \tau = -LH$ is the discrete analog of thermodynamic diffusion on a Riemannian manifold.

The operators that compute [[cyberank]] are the same operators that govern spatial structure in physical reality. The cybergraph is not a metaphor for a world. It is a world in the same mathematical sense that a Riemannian manifold with matter fields is a world — defined by its geometry, its dynamics, and its conserved quantities.

Every participant running the [[tri-kernel]] on the same authenticated graph state arrives at the same world. No rendering server. No consensus protocol for visual state. No negotiation over what the world looks like. The mathematics converges, and the world appears.

## references

- Fiedler, M. Algebraic connectivity of graphs. Czech Math Journal, 1973
- Chung, F. The heat kernel as the pagerank of a graph. PNAS, 2007
- Spielman, D. Spectral graph theory. Yale Lecture Notes
- Levin, D., Peres, Y., Wilmer, E. Markov chains and mixing times. AMS, 2009
- Brin, S., Page, L. The anatomy of a large-scale hypertextual web search engine. WWW, 1998
- Friston, K. The free-energy principle: a unified brain theory. Nature Reviews Neuroscience, 2010
- Ben-Sasson, E. et al. Scalable, transparent arguments of knowledge. CRYPTO, 2018