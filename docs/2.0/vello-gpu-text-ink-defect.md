# 缺陷：vello GPU presenter 不绘制裸 `Text` 节点（macOS/Metal）

> 发现日期：2026-09-28 · 发现途径：给 `floom::run_desktop` 做 Metal 冒烟截图
> 状态：**未修，已定位到分层边界** · 影响面：所有走 `VelloGpuPresenter` 的 v2 应用

## 症状

窗口正常创建、Metal 适配器可用、按钮/渐变/路径等节点都画出墨迹，但**任何不在容器背景上的 `Text` 节点不可见**。

对照截图（同一台 M4 Pro，同一个 presenter）：

| 示例 | 可见 | 不可见 |
|---|---|---|
| `floom-window --features real` | 渐变 `Image`、蓝色 `Button` | `Text::new("clicked 0 times")` |
| `floom-run-desktop`（本次新增） | 蓝色 `Button` | `Text::new("run_desktop: a real winit window")` |

`floom-window` 是本次改动之前就存在的示例，**同样不可见** —— 所以这不是 `run_desktop`
引入的回归，而是先存在的后端缺陷。

## 已排除的原因（有证据）

- **不是「没进 scene」**：headless 探针显示节点存在且带字形资源
  `SceneNode::Text { origin: (0,0), size: 28.01×14.0, glyphs: ResourceId(raw 4294967296), color: rgba(0,0,0,255) }`，
  同帧 `painted=3`。
- **不是颜色问题**：裸 `Text` 颜色是**黑** `rgba(0,0,0,255)`，按钮内 `Text` 是白
  `rgba(255,255,255,255)` —— 而按钮的白字**能看见**，黑字反而看不见，故与明暗无关。
- **不是资源缺失**：`ResourceKind::GlyphRun` 能解引用出 `runs`（软件渲染器
  `glyphs_drawn > 0` 的既有用例通过，见 `crates/floom-renderer-software/src/lib.rs:1870`
  与 `:1934` 两个测试，它们断言了像素级 ink）。
- **不是编码器不发字形命令**：`gpu.rs:1418` 的 `SceneNode::Text` 分支会
  `draw_glyphs(...).transform(...).brush(...)`，并在 `:1451` 计 `stats.glyph_runs += 1`。
  `floom-window` 运行日志 `frame 300: rasters 19557 (partial 1)` 说明确有光栅化发生。

## 像素证据

`screencapture` 抓 480×272 窗口（Retina 2x → 960×544），标题栏以下为内容区。
按 scene dump 预测裸 `Text` 应落在逻辑 (0,0)-(28,14)，即 2x 下约 y=64..92、x=8..64：

- 该矩形内**仅有 x=0 一列**存在暗像素（那是窗口左边缘，不是字形）。
- 按钮蓝像素正常落在 y=124..199。

即：文字**位置正确、颜色正确、资源正确，但没有墨迹**。

## 尚未区分的两个候选

1. **片段缓存把空内容固化了** —— `gpu.rs:1282` 的 `cached.fingerprint == fingerprint`
   命中后直接 `append` 缓存的 vello scene。日志里 `fragments reused 3` 说明缓存路径在用。
   若首次编码时 surface 尚未就绪，缓存的就是空场景，之后指纹不变便一直复用空片段。
2. **Metal surface 上的 glyph 绘制本身无输出** —— `draw_glyphs` 需要已上传的字体；
   若 `run.font.data` 在 GPU 路径上被换成空 blob，`:1432` 的
   `if run.font.data.as_ref().is_empty() { continue; }` 会**静默跳过**且不计任何统计，
   正好解释「位置对、颜色对、没墨迹、且统计不报错」。

两者都符合现有证据，区分需要读 `run.font` 在 GPU 路径上的实际内容（探针）或
临时禁用片段缓存后重跑对比。

## 复现

```bash
cargo run -p floom-run-desktop                 # 本次新增的最小示例
osascript -e 'tell application "System Events" to get {position,size} of window 1'
screencapture -x -R<x>,<y>,480,272 /tmp/shot.png
# 或用既有示例
FLOOM2_WINDOW_IDLE=1 cargo run -p floom-window --features real
```

## 建议的修法方向

先加一条**会红的测试**：在 `crates/floom-renderer-vello` 里对「单个 `Text` 节点」
断言编码后 `stats.glyph_runs > 0` **且** 字形 draw 命令确实进入 vello scene。
现状下这条测试应当失败 —— 有了它，上面两个候选就能被二分，而不是靠读码推断。

> 注：`VelloGpuPresenter` 只实现 `Presenter`、且 `attach()` 需要真实 surface handles
> （headless surface 直接返回 `false`），所以 offscreen 单测无法直接统计 GPU 侧墨迹；
> 断言必须落在**编码产物**（`glyph_runs` 计数 / vello scene 命令）而非最终像素。
