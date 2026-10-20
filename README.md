# 微明 Glimmer

> 输入的不只是文字。

微明（Glimmer）是一个使用 **Rust** 开发的跨平台输入法。

[![观看微明 Glimmer 宣传视频](https://i.ytimg.com/vi/MIi0LcrmQmQ/maxresdefault.jpg)](https://www.youtube.com/watch?v=MIi0LcrmQmQ)

[观看 45 秒宣传视频](https://www.youtube.com/watch?v=MIi0LcrmQmQ)：了解候选译词、本地整句、习惯学习与隐私保护。视频中的输入界面为动画演示示意。

它的目标不只是「把拼音转换成中文」，而是让输入本身成为一种轻量、持续、几乎没有额外负担的语言学习方式。

当你输入文字时，微明会在候选词旁边显示一条简洁的目标语言译文。

例如，当你的主要语言是中文、正在学习英语时：

```text
1  开发            development
2  编程            programming
3  架构            architecture
4  编译            compile
5  语言            language
```

候选词仍然是输入的主体，翻译只作为较小、较浅的辅助信息存在。

**一次只学习一种语言。** 微明不会在一个候选项旁边同时塞入英语、日语、韩语、德语。保持输入体验干净，比堆砌信息更重要。

- 官网：[glimmerinput.app](https://glimmerinput.app)
- 下载：[glimmerinput.app/download](https://glimmerinput.app/download)（macOS、Windows）
- 文档：[glimmerinput.app/docs](https://glimmerinput.app/docs)（安装、按键、设置、数据与隐私）
- 反馈：[GitHub Issues](https://github.com/OpenFabrica/glimmer/issues/new/choose)

---

## 为什么叫「微明」

「微明」出自《道德经》第三十六章：「是谓微明。」一点很细的亮，微小但清楚。

输入法就该是这样：它藏在文字后面，不抢眼、不打扰，只在你需要的地方给一点光。英文名 Glimmer 也是同一个意思。

---

## 来源与致谢

微明派生自 [青简 Qingjian](https://github.com/qingjian-team/qingjian)（GPL-3.0-or-later），引擎、平台壳与数据工具链均来自该项目，
在此感谢原作者与贡献者。微明沿用同一许可证，原有版权声明全部保留；本仓库的改动同样以 GPL-3.0-or-later 发布。

---

## 核心理念

> 学习语言为什么一定要专门打开一个学习软件？

聊天、写代码、搜索、记笔记、写文档、发邮件……大量时间其实都花在输入文字上。

如果这些每天发生数百次的输入行为，本身就能顺便提供一点语言反馈，语言学习就可以从「专门腾时间学习」变成日常行为的一部分。

**输入的时候，顺便多认识一个词。**

不打断。不弹题。不强迫记忆。只是悄悄地把翻译放在那里。

---

## 语言模式

用户可以设置：

```text
Primary Language: 中文
Learning Language: English
```

切换学习语言为日语后，同样的候选会显示：

```text
开发                開発
学习                学習
语言                言語
```

未来支持其他输入模式后，也可以反过来：

```text
こんにちは          你好
日本語              日语
```

核心原则不变：**一个候选词，只显示一种辅助语言。**

---

## 平台

微明从一开始就按跨平台架构设计：核心输入引擎平台无关，各平台只负责接入系统输入接口与候选窗口。

```text
macOS    → Input Method Kit (IMK)
Windows  → Text Services Framework (TSF)
Linux    → IBus / Fcitx
```

开发顺序是 macOS 优先；Windows 版已进入内测（TSF 文本服务 + 独立的输入引擎进程）。

---

## 不打算做什么

微明暂时不准备成为一个「大而全」的语言学习软件。它不会：

- 在候选框塞入五六种语言
- 每输入几个词就弹出测试
- 强制用户背单词
- 用复杂 UI 干扰正常输入
- 为了学习功能牺牲输入效率

输入法首先必须是一个好用的输入法。语言学习建立在这个前提之上。

如果用户需要思考「我现在到底是在打字还是在背单词」，那微明大概就设计错了。

---

## Philosophy

**输入优先。**

**学习自然发生。**

**平台只是壳，Core 才是微明。**

---

## 隐私

**微明不上传任何数据。** 拼音转换、词库、学习、释义全部在本机完成，没有账号，没有统计上报。
云联想（缺省关闭）打开后，请求直接从你的电脑发到你自己填写的 AI 服务商，不经过作者；输入日志只写在本机，可以随时关闭和清空。
细节见文档 [数据与日志](https://glimmerinput.app/docs/help/data-and-logs)。

---

## 许可

代码以 **GPL-3.0-or-later** 发布（见 [LICENSE](LICENSE)）：可以自由使用、修改与再分发，修改后分发须同样开源。
「微明」名字与 logo 不在授权范围内。微明在官方渠道免费；若你为获得它向他人付费，你被骗了。

随包数据（词库、五笔码表、语言模型、释义表、emoji、英文词表、词汇等级）各自遵循来源的许可证，清单见 [docs/design/landscape.md](docs/design/landscape.md)，偏好设置「关于」页也列了一份。
其中三份五笔码表（字根与编码规则由王永民发明）的来源是：

- 86 五笔：[rime-wubi](https://github.com/rime/rime-wubi)（LGPL-3.0；极点五笔血统，86 版的编码规则已进入公有领域），见 [assets/wubi/wubi86/](assets/wubi/wubi86/README.md)。
- 98 五笔：[98wubi](https://github.com/yanhuacuo/98wubi) 的含词码表（LGPL-3.0），见 [assets/wubi/wubi98/](assets/wubi/wubi98/README.md)。
- 新世纪五笔：[wubixinshiji](https://github.com/GuoBinyong/wubixinshiji)（郭斌勇整理，`AUTHORS` 声明 LGPL），见 [assets/wubi/wubixsj/](assets/wubi/wubixsj/README.md)。

---

## 参与开发

技术架构、设计决定、路线图与工程记录见 [`docs/`](docs/)；改代码前先看 [开发约定](docs/contributing.md)。
欢迎提 issue 与 PR，PR 模板里有合并前清单。

---

## Status

测试版，自用中，正在给少数测试者打包。API、项目结构和功能设计都可能发生较大变化。

---

<p align="center">
  <strong>微明 Glimmer</strong><br/>
  输入的不只是文字。
</p>
