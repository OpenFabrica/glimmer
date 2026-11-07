use glimmer_core::FuzzyRules;
use glimmer_platform::Config;
use objc2::rc::Retained;
use objc2::{MainThreadMarker, sel};
use objc2_app_kit::{NSControlStateValueOff, NSControlStateValueOn, NSMenu, NSMenuItem};
use objc2_foundation::NSString;

use super::MenuAction;
use super::target::MenuTarget;

/// 「英文辅助」菜单项的标题（不含快捷键提示）。
const ENGLISH_ASSIST_TITLE: &str = "英文补全、纠错、预测";

/// 菜单本体与需要按状态刷新的那几项。
pub struct InputMenu {
    /// 菜单。挂到状态项和 IMK `menu` 回调的是同一个对象。
    menu: Retained<NSMenu>,

    /// 「云联想」勾选项。
    cloud: Retained<NSMenuItem>,

    /// 「英文补全、纠错、预测」勾选项，标题带开关快捷键。
    english_assist: Retained<NSMenuItem>,

    /// 模糊音子菜单的九条勾选项，顺序同 [`FuzzyRules::NAMES`]。
    fuzzy: Vec<Retained<NSMenuItem>>,

    /// 版本行；配置文件解析失败时标题换成错误提示（不另加一行，菜单项运行中不能隐藏 / 显示，见 [`action_item`]）。
    about: Retained<NSMenuItem>,

    /// 版本行平时的标题。
    about_title: String,

    /// 查到的新版本号；有它时版本行后面带「新版本 x 可用」。
    update: Option<String>,

    /// 当前输入方案的展示行（「输入方案：拼音」/「输入方案：86 五笔」）；常驻只改标题，不隐藏 / 显示切换（见 [`action_item`]）。
    scheme: Retained<NSMenuItem>,

    /// 所有条目的 target，要和菜单活得一样久。
    _target: Retained<MenuTarget>,
}

impl InputMenu {
    pub fn new(mtm: MainThreadMarker, version: &str) -> Self {
        let target = MenuTarget::new(mtm);
        let menu = NSMenu::new(mtm);
        // 不让 AppKit 按响应链判断可用性：它找不到 target 就会把整份菜单灰掉
        menu.setAutoenablesItems(false);

        let scheme = action_item(mtm, "输入方案：拼音", None, &target);
        scheme.setEnabled(false);
        menu.addItem(&scheme);
        let cloud = action_item(mtm, "云联想", Some(MenuAction::ToggleCloud), &target);
        menu.addItem(&cloud);
        let english_assist = action_item(
            mtm,
            ENGLISH_ASSIST_TITLE,
            Some(MenuAction::ToggleEnglishAssist),
            &target,
        );
        menu.addItem(&english_assist);

        let fuzzy_menu = NSMenu::new(mtm);
        fuzzy_menu.setAutoenablesItems(false);
        let fuzzy: Vec<_> = FuzzyRules::NAMES
            .iter()
            .enumerate()
            .map(|(index, name)| {
                let item = action_item(
                    mtm,
                    &fuzzy_label(name),
                    Some(MenuAction::ToggleFuzzy(index)),
                    &target,
                );
                fuzzy_menu.addItem(&item);
                item
            })
            .collect();
        let fuzzy_parent = action_item(mtm, "模糊音", None, &target);
        fuzzy_parent.setSubmenu(Some(&fuzzy_menu));
        menu.addItem(&fuzzy_parent);

        menu.addItem(&NSMenuItem::separatorItem(mtm));
        menu.addItem(&action_item(
            mtm,
            "偏好设置…",
            Some(MenuAction::OpenPreferences),
            &target,
        ));
        menu.addItem(&action_item(
            mtm,
            "打开日志目录",
            Some(MenuAction::OpenLogs),
            &target,
        ));
        menu.addItem(&action_item(
            mtm,
            "检查更新…",
            Some(MenuAction::CheckUpdate),
            &target,
        ));
        menu.addItem(&NSMenuItem::separatorItem(mtm));

        let about_title = format!("微明 {version}");
        let about = action_item(mtm, &about_title, None, &target);
        about.setEnabled(false);
        menu.addItem(&about);

        Self {
            menu,
            cloud,
            english_assist,
            fuzzy,
            about,
            about_title,
            update: None,
            scheme,
            _target: target,
        }
    }

    /// 记下查到的新版本（`None` 是已最新），版本行立刻跟着改。
    pub fn set_update(&mut self, version: Option<&str>) {
        self.update = version.map(str::to_owned);
        self.about
            .setTitle(&NSString::from_str(&self.version_title()));
    }

    /// 版本行标题：「微明 0.1.7」，有新版本时「微明 0.1.7（新版本 0.1.8 可用）」。
    fn version_title(&self) -> String {
        match &self.update {
            Some(version) => format!("{}（新版本 {version} 可用）", self.about_title),
            None => self.about_title.clone(),
        }
    }

    /// 给状态项 / IMK 回调用的菜单对象。
    pub fn ns_menu(&self) -> Retained<NSMenu> {
        self.menu.clone()
    }

    /// 按当前配置刷新勾选状态。`cloud_active` 是 Engine 里真接上了 Predictor：
    /// 配置开了但没接上（多半是没密钥）时不打勾，标题说明原因，不能显示开了实际没开。
    /// `scheme` 同理是 Engine 里真生效的方案名（码表缺了配置开着也不算五笔），顶上那行照它写；
    /// 空串是缺省的单开全拼，那时写「拼音」。
    pub fn sync(&self, config: &Config, cloud_active: bool, scheme: &str, error: Option<&str>) {
        let scheme = if scheme.is_empty() { "拼音" } else { scheme };
        self.scheme
            .setTitle(&NSString::from_str(&format!("输入方案：{scheme}")));
        let title = match (config.predict.enabled, cloud_active) {
            (true, false) => "云联想（启用失败，见日志）",
            _ => "云联想",
        };
        self.cloud.setTitle(&NSString::from_str(title));
        set_checked(&self.cloud, cloud_active);
        // 快捷键提示写进标题而不是 keyEquivalent：菜单项建好后只能改标题与勾选（见 action_item），键改了也要跟上
        let assist = match config.shortcut.english_assist_keys() {
            Some(combo) => format!("{ENGLISH_ASSIST_TITLE}（{}）", combo.label()),
            None => ENGLISH_ASSIST_TITLE.to_owned(),
        };
        self.english_assist.setTitle(&NSString::from_str(&assist));
        set_checked(&self.english_assist, config.general.english_candidates);
        for (item, name) in self.fuzzy.iter().zip(FuzzyRules::NAMES) {
            set_checked(item, config.fuzzy.is_on(name));
        }
        let about = match error {
            Some(message) => format!("配置文件有错误：{message}"),
            None => self.version_title(),
        };
        self.about.setTitle(&NSString::from_str(&about));
    }
}

/// 建一个菜单项。`action` 为 `None` 的是纯展示项（子菜单父项、方案行、版本行）。
///
/// 菜单项建好之后**只能改标题与勾选**，不能运行中 `setHidden` / 增删：IMK 把菜单同步给系统时按项记录，
/// 没有 action 的项记成 NULL，这一项的显示状态一变它释放旧记录就 `CFRelease(NULL)` 崩（macOS 27，栈在
/// `_copySynchronizedActions:withMenuItems:`）；而给展示项也绑上选择器，系统输入源菜单又会整份不显示我们的条目。
fn action_item(
    mtm: MainThreadMarker,
    title: &str,
    action: Option<MenuAction>,
    target: &MenuTarget,
) -> Retained<NSMenuItem> {
    // SAFETY: 选择器与 MenuTarget / 控制器上定义的 `menuAction:` 一致，签名 (id) -> void
    let item = unsafe {
        NSMenuItem::initWithTitle_action_keyEquivalent(
            mtm.alloc(),
            &NSString::from_str(title),
            action.map(|_| sel!(menuAction:)),
            &NSString::from_str(""),
        )
    };
    if let Some(action) = action {
        unsafe { item.setTarget(Some(target)) };
        item.setTag(action.tag());
    }
    item
}

fn set_checked(item: &NSMenuItem, on: bool) {
    item.setState(if on {
        NSControlStateValueOn
    } else {
        NSControlStateValueOff
    });
}

/// `an_ang` → `an = ang`。
fn fuzzy_label(name: &str) -> String {
    name.replace('_', " = ")
}
