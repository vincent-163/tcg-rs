# 仓库指南（Repository Guidelines）

## 项目结构与模块组织
- core/: IR 类型、操作码、临时变量、标签与 IR builder（gen_*）。
- backend/: 活跃分析、约束系统、寄存器分配与 x86-64 代码生成。
- decode/: QEMU 风格 .decode 文件解析器与 Rust 代码生成器。
- frontend/: 客户指令解码框架与 RISC-V RV64IMAFDC 前端。
- exec/: MTTCG 执行循环、TB 缓存/链路、SharedState/PerCpuState。
- linux-user/: ELF 加载、guest 地址空间、syscall 仿真、tcg-riscv64 运行器。
- tests/: 单元测试、后端回归、前端翻译、差分测试、MTTCG、linux-user 端到端。
- docs/: 设计、IR ops、后端、测试体系与代码风格文档。

## 构建、测试与开发命令
    cargo build                 # 构建全部 crate
    cargo test                  # 运行全量测试
    cargo test -p tcg-tests     # 仅运行后端与集成测试
    cargo clippy -- -D warnings # 静态检查
    cargo fmt --check           # 格式检查
不使用 CI/CD 自动化；构建、测试、发布均为手工操作。

## 编码风格与命名规范
- 默认缩进 4 空格；若文件已有风格则保持一致。
- Rust 命名：模块与函数使用 snake_case，类型使用 CamelCase。
- 注释尽量少且用英文，只解释非显然逻辑。
- 变更以“小而明确”为优先，默认清理过时代码。

## 测试指南
- 使用 Rust 内置测试框架（#[test]）。
- 测试命名采用 test_*，保持用例窄、确定性强。
- 后端回归放在 tests/src/backend/，IR/TB 执行用例放在
  tests/src/integration/。
- 前端翻译测试放在 tests/src/frontend/，差分测试（difftest）
  放在 tests/src/frontend/difftest.rs。
- 执行循环与 MTTCG 测试放在 tests/src/exec/。
- linux-user 端到端测试放在 tests/src/linux_user/。
- 修复缺陷时必须补充覆盖该场景的回归测试。

## 提交与 PR 指南

Commit message 必须使用英文编写。格式如下：

```
module: subject

具体修改内容的详细说明。

Signed-off-by: Name <email>
```

- Subject 行总长度不超过 72 字符
- Body 每行不超过 80 字符
- `.md` 文档文件不受 80 列行宽限制

## 角色职责与质量要求
- 主要职责：审查与 review 代码、编写测题、把关代码质量。
- 优先发现行为风险、回归可能与测试缺口，并给出可复现依据。

## 文档与参考
- 行为变化需同步更新 docs/。
- 对齐 QEMU 行为时，注明对应源码位置与约束来源。

## SPEC2006 测试指南

### 测试前准备
运行 SPEC2006 测试前必须完成以下步骤：

1. **提交所有工作区更改**
   ```bash
   git add -A
   git commit -m "pre-spec2006: save current work"
   ```

2. **记录当前 commit hash**
   ```bash
   git rev-parse HEAD
   ```

3. **重新构建 release 版本**
   ```bash
   cargo build --release --features llvm --bin tcg-aarch64
   ```

### 运行 SPEC2006 INT JIT 测试

```bash
# 并行运行所有 INT 测试（推荐，带 profiling）
./run-spec2006int-jit.sh parallel

# 串行运行所有 INT 测试
./run-spec2006int-jit.sh serial
```

**JIT 测试特性：**
- 自动启用 profiling (`TCG_PROFILE=1`)
- 收集 profile 到 `cache/profiles/` 目录
- 每个 testcase 完成后立即在日志中记录结果
- Profile 文件可用于后续的 AOT 编译

### 运行 SPEC2006 INT AOT 测试

```bash
# 并行运行所有 INT 测试
./run-spec2006int-aot.sh parallel

# 串行运行
./run-spec2006int-aot.sh serial
```

### 日志记录要求

每次完整运行 SPEC2006 测试后，必须将结果记录到 `SPEC2006LOG.md`：

1. **脚本会自动记录以下内容到 SPEC2006LOG.md**：
   - 运行日期和时间
   - 当前 git commit hash
   - 通过的测试列表
   - 失败的测试列表
   - 结果目录路径

2. **如果手动运行，使用以下命令追加日志**：
   ```bash
   cat >> SPEC2006LOG.md << 'EOF'
   ## YYYY-MM-DD HH:MM:SS
   - Commit: <commit-hash>
   - Mode: JIT/AOT
   - Passed: <count>
   - Failed: <count>
   - Failed tests: <list>
   - Results: <path-to-results>
   EOF
   ```

3. **日志原则**：
   - 所有测试输出必须写入日志文件，不要直接打印到控制台
   - 日志文件位置：`spec2006int-*-results-<timestamp>/logs/`
   - 汇总文件：`spec2006int-*-results-<timestamp>/summary.txt`

### 查看测试结果

```bash
# 查看特定 tag 的状态
./tools/spec/specint-status.sh <tag>

# 实时监控队列
./tools/spec/watch-specint.sh <tag>

# 重新运行 compare 阶段
./tools/spec/rerun-compare.sh <run-dir>
```
