# NESium CLI

NES 模拟器的命令行界面版本。

## 功能

- 从命令行直接加载和运行 NES ROM 文件
- 使用 ASCII 艺术显示游戏画面
- 键盘控制支持
- 实时 FPS 显示

## 构建

```bash
cargo build -p nesium-cli
```

构建完成后，可执行文件位于：
- macOS/Linux: `target/debug/nesium-cli`
- Windows: `target/debug/nesium-cli.exe`

## 使用方法

```bash
./target/debug/nesium-cli <rom_path>
```

### 参数

- `rom_path`: NES ROM 文件的路径

## 控制方式

| 按键 | 功能 |
|------|------|
| W | 上 |
| S | 下 |
| A | 左 |
| D | 右 |
| J | A 按钮 |
| K | B 按钮 |
| L | Select |
| ; | Start |
| Q | 退出 |

## 要求

- 终端窗口大小至少为 80x40 字符
- 支持的颜色模式：终端需要支持基本的颜色显示

## 屏幕显示

CLI 模式使用 ASCII 艺术来显示游戏画面：
- 原始分辨率：256x240
- ASCII 显示：64x60（缩小以适应终端）
- 使用不同字符表示亮度：
  - `@` 最亮
  - `#` 较亮
  - `=` 中等
  - `-` 较暗
  - ` ` 最暗

## 技术细节

- 使用 `nesium-core` 作为核心模拟器
- 使用 `ratatui` 提供终端 UI
- 使用 `crossterm` 处理键盘输入
- 通过 `try_render_buffer()` 获取帧数据
- 将 RGB555 格式转换为灰度 ASCII 字符
