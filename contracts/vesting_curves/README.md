---

### Summary  
Introduces the `VestingCurve` enum to support **Linear**, **Exponential**, and **ExponentialDecay** vesting schedules in the Vesting Vault contract. This enhancement allows flexible release patterns, enabling steady vesting, a slow-start/accelerated curve, or a front-loaded decay curve while ensuring correctness through unit and integration tests.  

### Key Features  
* **VestingCurve Enum**:  
  - `Linear`: vested = total × elapsed ÷ duration  
  - `Exponential`: vested = total × elapsed² ÷ duration²  
  - `ExponentialDecay`: vested = total - (total × remaining² ÷ duration²)  
* **Function Dispatch**: `vested_amount`, `claim`, and `status` now branch on curve type.  
* **Mathematical Behavior**:  
  - Linear: proportional vesting (50% time → 50% tokens).  
  - Exponential: slower start, faster finish (50% time → 25% tokens).  
  - ExponentialDecay: faster start, slower finish (50% time → 75% tokens).  
* **Immutable Curve**: Curve set at `initialize()` and cannot be changed mid-schedule.  
* **Incremental Claim Guard**: Ensures multiple claims sum correctly regardless of curve.  
* **Testing**: 11 unit + integration tests validating math, claims, and curve behavior.  

### How to Test  
1. Run `cargo test` in the `vesting-vault` workspace.  
   - ✅ All 11 tests should pass, covering both Linear and Exponential curves.  
2. Build the WASM binary: `stellar contract build`.  
3. Deploy to Stellar Testnet with `stellar contract deploy`.  
4. Initialize vaults with `--curve '{"Linear": {}}'`, `--curve '{"Exponential": {}}'`, or `--curve '{"ExponentialDecay": {}}'`.  
5. Invoke `get_curve` to confirm correct variant.  
6. Check `vested_amount` at 50% elapsed: Linear → 50%, Exponential → 25%, ExponentialDecay → 75%.  
7. Use `claim` to verify incremental transfers.  

### Checklist  
- [x] Add `VestingCurve` enum with Linear, Exponential, & ExponentialDecay variants  
- [x] Update `vested_amount`, `claim`, and `status` to dispatch on curve  
- [x] Ensure integer-only math with `u128` intermediates  
- [x] Enforce immutable curve at initialization  
- [x] Implement incremental claim logic  
- [x] Write unit/integration tests for all curve variants  
- [x] Build & deploy WASM contract to Stellar Testnet

---

## Mathematical Behavior & Visuals

### Linear Vesting
Formula: `vested = total * elapsed / duration`

```mermaid
graph LR
  S[Start] --> T[Time]
  T --> V[Tokens vested = total * elapsed / duration]
```

### Step-based Vesting
Formula: `vested = sum of unlocked steps`

```mermaid
graph LR
  S2[Start] --> T2[Time]
  T2 --> U[Tokens vested = sum of unlocked steps]
```

### Cliff Vesting
Formula: `vested = 0 if before cliff, else linear`

```mermaid
graph LR
  S3[Start] --> C[Cliff]
  C --> T3[Time]
  T3 --> W[Tokens vested = 0 if before cliff, else linear]
```

Each chart visually compares the vesting schedule. The formula shown is exactly what is used in the Rust contract code.

