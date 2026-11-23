//! The app's words in English, Vietnamese, Chinese (simplified) and French. Code keeps the English
//! text and asks `t` for the chosen language's; a phrase with no translation stays English.
use chrono::{Datelike, NaiveDateTime, Weekday};
use std::sync::atomic::{AtomicU8, Ordering};

pub const EN: usize = 0;
pub const VI: usize = 1;
pub const ZH: usize = 2;
pub const FR: usize = 3;
/// Each language named in itself, for the Settings drop-down.
pub const NAMES: [&str; 4] = ["English", "Tiếng Việt", "中文 (简体)", "Français"];
/// The same languages named in English, for asking Gemini to translate into them.
pub const ENGLISH_NAMES: [&str; 4] = ["English", "Vietnamese", "Simplified Chinese", "French"];

static LANG: AtomicU8 = AtomicU8::new(EN as u8);
/// Tests that switch or read the language hold this, so they don't see each other's choice.
#[cfg(test)]
pub static TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

pub fn set(lang: usize) {
    LANG.store(lang.min(NAMES.len() - 1) as u8, Ordering::Relaxed);
}

pub fn lang() -> usize {
    LANG.load(Ordering::Relaxed) as usize
}

/// Windows' display language when it is one of ours, otherwise English.
pub fn system_default() -> usize {
    let id = unsafe { windows_sys::Win32::Globalization::GetUserDefaultUILanguage() };
    match id & 0x3ff {
        0x2a => VI,
        0x04 => ZH,
        0x0c => FR,
        _ => EN,
    }
}

/// The phrase in the chosen language.
pub fn t(en: &str) -> &str {
    let l = lang();
    if l == EN {
        return en;
    }
    match TABLE.iter().find(|row| row.0 == en) {
        Some(row) => [row.0, row.1, row.2, row.3][l],
        None => en,
    }
}

/// A phrase with `{}` gaps, filled in order.
pub fn tr(en: &str, args: &[&dyn std::fmt::Display]) -> String {
    let mut out = String::new();
    let mut rest = t(en);
    for a in args {
        match rest.split_once("{}") {
            Some((head, tail)) => {
                out.push_str(head);
                out.push_str(&a.to_string());
                rest = tail;
            }
            None => break,
        }
    }
    out.push_str(rest);
    out
}

const DAYS: [[&str; 7]; 4] = [
    ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"],
    ["T2", "T3", "T4", "T5", "T6", "T7", "CN"],
    ["周一", "周二", "周三", "周四", "周五", "周六", "周日"],
    ["lun.", "mar.", "mer.", "jeu.", "ven.", "sam.", "dim."],
];
const DAY_LETTERS: [[&str; 7]; 4] = [
    ["M", "T", "W", "T", "F", "S", "S"],
    ["T2", "T3", "T4", "T5", "T6", "T7", "CN"],
    ["一", "二", "三", "四", "五", "六", "日"],
    ["L", "M", "M", "J", "V", "S", "D"],
];
const MONTHS: [[&str; 12]; 4] = [
    ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"],
    ["thg 1", "thg 2", "thg 3", "thg 4", "thg 5", "thg 6", "thg 7", "thg 8", "thg 9", "thg 10", "thg 11", "thg 12"],
    ["1月", "2月", "3月", "4月", "5月", "6月", "7月", "8月", "9月", "10月", "11月", "12月"],
    ["janv.", "févr.", "mars", "avr.", "mai", "juin", "juil.", "août", "sept.", "oct.", "nov.", "déc."],
];

/// "Fri 20 Nov 2026" in the chosen language.
pub fn date(d: NaiveDateTime) -> String {
    let l = lang();
    let day = DAYS[l][d.weekday().num_days_from_monday() as usize];
    let month = MONTHS[l][d.month0() as usize];
    match l {
        ZH => format!("{}年{}{}日 {day}", d.year(), month, d.day()),
        VI => format!("{day}, {} {month} {}", d.day(), d.year()),
        _ => format!("{day} {} {month} {}", d.day(), d.year()),
    }
}

/// The weekday's short head for the week calendar.
pub fn day_letter(w: Weekday) -> &'static str {
    DAY_LETTERS[lang()][w.num_days_from_monday() as usize]
}

/// (English, Tiếng Việt, 中文, Français). Gaps are `{}`, in the same order in every language.
#[rustfmt::skip]
const TABLE: &[(&str, &str, &str, &str)] = &[
    // Pages and their headers
    ("Music", "Nhạc", "音乐", "Musique"),
    ("Pomodoro", "Pomodoro", "番茄钟", "Pomodoro"),
    ("Water", "Nước", "喝水", "Eau"),
    ("Shelf", "Kệ", "收纳架", "Étagère"),
    ("To-do", "Việc cần làm", "待办", "À faire"),
    ("Prayer", "Cầu nguyện", "祷告", "Prière"),
    ("Widgets", "Tiện ích", "小组件", "Widgets"),
    ("Today", "Hôm nay", "今天", "Aujourd'hui"),
    ("Water ", "Nhắc ", "喝水", "Rappel "),
    ("Reminder", "uống nước", "提醒", "d'eau"),
    ("Todo", "Danh sách ", "待办", "Liste "),
    ("List", "việc", "清单", "de tâches"),
    // Moon and countdowns
    ("New Moon", "Trăng non", "新月", "Nouvelle lune"),
    ("Waxing Crescent", "Trăng lưỡi liềm đầu", "蛾眉月", "Premier croissant"),
    ("First Quarter", "Bán nguyệt đầu", "上弦月", "Premier quartier"),
    ("Waxing Gibbous", "Trăng khuyết đầu", "盈凸月", "Gibbeuse croissante"),
    ("Full Moon", "Trăng tròn", "满月", "Pleine lune"),
    ("Waning Gibbous", "Trăng khuyết cuối", "亏凸月", "Gibbeuse décroissante"),
    ("Last Quarter", "Bán nguyệt cuối", "下弦月", "Dernier quartier"),
    ("Waning Crescent", "Trăng lưỡi liềm cuối", "残月", "Dernier croissant"),
    ("New Year", "Năm mới", "新年", "Nouvel An"),
    ("Ramadan", "Ramadan", "斋月", "Ramadan"),
    ("Eid al-Fitr", "Lễ Eid al-Fitr", "开斋节", "Aïd el-Fitr"),
    ("Eid al-Adha", "Lễ Eid al-Adha", "宰牲节", "Aïd el-Kébir"),
    ("Indonesia's Independence Day", "Ngày Độc lập Indonesia", "印尼独立日", "Fête de l'indépendance indonésienne"),
    ("Christmas", "Giáng sinh", "圣诞节", "Noël"),
    ("Lunar New Year (Tết)", "Tết Nguyên Đán", "春节", "Nouvel An lunaire (Têt)"),
    ("Vietnam's National Day", "Quốc khánh Việt Nam", "越南国庆节", "Fête nationale du Vietnam"),
    // Widgets
    ("This week", "Tuần này", "本周", "Cette semaine"),
    ("Battery", "Pin", "电池", "Batterie"),
    ("Next prayer", "Giờ cầu nguyện tới", "下次祷告", "Prochaine prière"),
    ("Focus today", "Tập trung hôm nay", "今日专注", "Concentration du jour"),
    ("World clock", "Giờ thế giới", "世界时钟", "Horloge mondiale"),
    ("Moon", "Mặt trăng", "月相", "Lune"),
    ("Countdown", "Đếm ngược", "倒计时", "Compte à rebours"),
    ("NOTIFICATIONS", "THÔNG BÁO", "通知", "NOTIFICATIONS"),
    ("THIS WEEK", "TUẦN NÀY", "本周", "CETTE SEMAINE"),
    ("BATTERY", "PIN", "电池", "BATTERIE"),
    ("NEXT PRAYER", "GIỜ CẦU NGUYỆN TỚI", "下次祷告", "PROCHAINE PRIÈRE"),
    ("FOCUS TODAY", "TẬP TRUNG HÔM NAY", "今日专注", "CONCENTRATION"),
    ("WORLD CLOCK", "GIỜ THẾ GIỚI", "世界时钟", "HORLOGE MONDIALE"),
    ("MOON", "MẶT TRĂNG", "月相", "LUNE"),
    ("COUNTDOWN", "ĐẾM NGƯỢC", "倒计时", "COMPTE À REBOURS"),
    ("WATER", "NƯỚC", "喝水", "EAU"),
    ("{} tasks today", "{} việc hôm nay", "今天 {} 项任务", "{} tâches aujourd'hui"),
    ("Next: {}, {}", "Tiếp: {}, {}", "下一项：{}，{}", "Ensuite : {}, {}"),
    ("Nothing scheduled", "Chưa có lịch", "暂无安排", "Rien de prévu"),
    ("On battery", "Đang dùng pin", "使用电池", "Sur batterie"),
    ("Plugged in", "Đang cắm sạc", "已接通电源", "Branché"),
    ("Unplugged", "Đã rút sạc", "未接电源", "Débranché"),
    ("No battery", "Không có pin", "无电池", "Pas de batterie"),
    ("{}% lit", "Sáng {}%", "亮度 {}%", "Éclairée à {} %"),
    ("days to {}", "ngày tới {}", "天后{}", "jours avant {}"),
    ("of {} L", "trên {} L", "共 {} 升", "sur {} L"),
    // Pop-ups
    ("Round complete", "Hoàn thành một vòng", "本轮完成", "Cycle terminé"),
    ("Time for a {} min break", "Nghỉ {} phút nhé", "休息 {} 分钟吧", "Pause de {} min"),
    ("Focus done", "Xong phiên tập trung", "专注完成", "Concentration terminée"),
    ("Break over", "Hết giờ nghỉ", "休息结束", "Fin de la pause"),
    ("Back to focus, {} min", "Quay lại tập trung, {} phút", "回到专注，{} 分钟", "On s'y remet, {} min"),
    ("Time to Refill", "Đến lúc rót thêm nước", "该加水了", "Il faut remplir"),
    ("Your bottle is probably empty", "Bình nước có lẽ đã cạn", "你的水瓶可能空了", "Votre gourde est sans doute vide"),
    ("Time to Drink", "Đến giờ uống nước", "该喝水了", "C'est l'heure de boire"),
    ("Task Time!", "Đến giờ làm việc!", "任务时间到！", "C'est l'heure !"),
    ("Time's Up!", "Hết giờ!", "时间到！", "Temps écoulé !"),
    ("In {} minutes", "Còn {} phút", "{} 分钟后", "Dans {} minutes"),
    ("It's time to pray", "Đến giờ cầu nguyện", "祷告时间到了", "C'est l'heure de la prière"),
    ("Charging", "Đang sạc", "正在充电", "En charge"),
    ("Charger connected", "Đã cắm sạc", "已连接充电器", "Chargeur branché"),
    ("Charger unplugged", "Đã rút sạc", "已拔下充电器", "Chargeur débranché"),
    ("Battery critical", "Pin rất yếu", "电量极低", "Batterie critique"),
    ("Plug in now", "Cắm sạc ngay", "请立即充电", "Branchez maintenant"),
    ("Low battery", "Pin yếu", "电量低", "Batterie faible"),
    ("Plug in soon", "Sắp cần cắm sạc", "请尽快充电", "Branchez bientôt"),
    ("Fully charged", "Đã sạc đầy", "已充满", "Chargée"),
    ("You can unplug", "Có thể rút sạc", "可以拔掉电源了", "Vous pouvez débrancher"),
    ("Volume", "Âm lượng", "音量", "Volume"),
    ("Brightness", "Độ sáng", "亮度", "Luminosité"),
    ("Connected", "Đã kết nối", "已连接", "Connecté"),
    ("Connected · {}%", "Đã kết nối · {}%", "已连接 · {}%", "Connecté · {} %"),
    ("Disconnected", "Mất kết nối", "已断开", "Déconnecté"),
    ("No internet access", "Không có Internet", "无法访问互联网", "Pas d'accès à Internet"),
    ("Caps Lock", "Caps Lock", "大写锁定", "Verr. Maj"),
    ("On", "Bật", "开", "Activé"),
    ("Off", "Tắt", "关", "Désactivé"),
    ("Input language", "Ngôn ngữ nhập", "输入语言", "Langue de saisie"),
    ("Refilled", "Đã rót", "已加水", "Rempli"),
    ("Drank", "Đã uống", "已喝", "Bu"),
    ("Snooze", "Để sau", "稍后", "Plus tard"),
    ("to", "đến", "至", "à"),
    ("until {}", "đến {}", "至 {}", "jusqu'à {}"),
    ("Calendar · in {} min", "Lịch · còn {} phút", "日历 · {} 分钟后", "Agenda · dans {} min"),
    ("Not there any more", "Không còn ở đó", "已不在原处", "N'est plus là"),
    ("Moved or deleted · right-click to remove", "Đã bị chuyển hoặc xóa · click phải để gỡ", "已移动或删除 · 右键移除", "Déplacé ou supprimé · clic droit pour retirer"),
    ("Click to copy · drag into a chat · right-click to remove", "Click để chép · kéo vào khung chat · click phải để xóa", "点击复制 · 拖入聊天 · 右键删除", "Clic pour copier · glisser dans un chat · clic droit pour retirer"),
    ("Drank {} ml · ", "Đã uống {} ml · ", "已喝 {} 毫升 · ", "Bu {} ml · "),
    ("Undo", "Hoàn tác", "撤销", "Annuler"),
    ("Done", "Xong", "完成", "Terminé"),
    ("Later", "Để sau", "稍后", "Plus tard"),
    ("Focus", "Tập trung", "专注", "Concentration"),
    ("Break", "Nghỉ", "休息", "Pause"),
    // Music
    ("Nothing playing", "Không có nhạc", "未在播放", "Rien en lecture"),
    ("Play music in any app", "Phát nhạc ở bất kỳ ứng dụng nào", "在任意应用中播放音乐", "Lancez de la musique dans une appli"),
    ("Loading lyrics…", "Đang tải lời bài hát…", "正在加载歌词…", "Chargement des paroles…"),
    ("Lyrics show here while music plays", "Lời bài hát hiện ở đây khi phát nhạc", "播放音乐时歌词显示在这里", "Les paroles s'affichent ici pendant la lecture"),
    ("No lyrics found for this song", "Không tìm thấy lời bài hát này", "未找到这首歌的歌词", "Aucune parole trouvée pour ce titre"),
    ("From LRCLIB, a community-contributed lyrics database", "Từ LRCLIB, kho lời bài hát do cộng đồng đóng góp", "来自社区共建的歌词库 LRCLIB", "Depuis LRCLIB, une base de paroles communautaire"),
    // Pomodoro
    ("Focus · {} of {}", "Tập trung · {} / {}", "专注 · 第 {} / {} 轮", "Concentration · {} sur {}"),
    ("Short break", "Nghỉ ngắn", "短休息", "Pause courte"),
    ("Long break", "Nghỉ dài", "长休息", "Pause longue"),
    ("until done", "là xong", "结束", "pour finir"),
    ("This round", "Vòng này", "本轮", "Ce cycle"),
    ("Pause", "Tạm dừng", "暂停", "Pause"),
    ("Start", "Bắt đầu", "开始", "Démarrer"),
    ("Resume", "Tiếp tục", "继续", "Reprendre"),
    ("Long", "Dài", "长", "Longue"),
    ("{} · {} sessions", "{} · {} phiên", "{} · {} 轮", "{} · {} sessions"),
    // Water
    ("Goal reached", "Đã đạt mục tiêu", "已达成目标", "Objectif atteint"),
    ("Reminders off", "Đã tắt nhắc", "提醒已关闭", "Rappels désactivés"),
    ("next {}", "lần tới {}", "下次 {}", "prochain {}"),
    ("of your goal", "mục tiêu", "目标", "de l'objectif"),
    ("Today · ", "Hôm nay · ", "今天 · ", "Aujourd'hui · "),
    ("Drink", "Uống", "喝水", "Boire"),
    ("Settings", "Cài đặt", "设置", "Réglages"),
    ("Goal", "Mục tiêu", "目标", "Objectif"),
    ("Bottle", "Bình", "水瓶", "Gourde"),
    ("Hours", "Giờ", "时段", "Heures"),
    ("{} reminders · {} refills · {} ml each", "{} lần nhắc · {} lần rót · mỗi lần {} ml", "{} 次提醒 · {} 次加水 · 每次 {} 毫升", "{} rappels · {} remplissages · {} ml chacun"),
    ("Sure?", "Chắc chưa?", "确定？", "Sûr ?"),
    ("Reset", "Đặt lại", "重置", "Réinitialiser"),
    // Shelf
    ("Clear", "Xoá hết", "清空", "Effacer"),
    ("Release to add", "Thả ra để thêm", "松开以添加", "Relâchez pour ajouter"),
    ("Drop files here", "Thả tệp vào đây", "将文件拖到这里", "Déposez des fichiers ici"),
    ("Drag out anytime · click to open", "Kéo ra bất cứ lúc nào · bấm để mở", "随时拖出 · 点击打开", "Glissez-les dehors · cliquez pour ouvrir"),
    ("Click to open · drag out · right-click to remove", "Bấm để mở · kéo ra · bấm phải để xoá", "点击打开 · 拖出 · 右键移除", "Cliquer : ouvrir · glisser : sortir · clic droit : retirer"),
    ("Clipboard history is off", "Lịch sử clipboard đang tắt", "剪贴板历史已关闭", "L'historique du presse-papiers est désactivé"),
    ("Turn it on in Settings (right-click the notch)", "Bật trong Cài đặt (bấm phải vào notch)", "在设置中开启（右键点击刘海）", "Activez-le dans Réglages (clic droit sur l'encoche)"),
    ("Copy some text", "Hãy sao chép một đoạn chữ", "复制一些文字", "Copiez du texte"),
    ("It will wait here, ready to paste again", "Nó sẽ nằm đây, sẵn sàng để dán lại", "它会留在这里，随时再次粘贴", "Il vous attendra ici, prêt à être recollé"),
    ("Copied", "Đã chép", "已复制", "Copié"),
    ("{} items · click to copy · drag into a chat", "{} mục · bấm để chép · kéo vào khung chat", "{} 项 · 点击复制 · 拖入聊天", "{} éléments · cliquer pour copier · glisser dans un chat"),
    // To-do
    ("Task Done Today", "Việc xong hôm nay", "今日已完成", "Tâches faites aujourd'hui"),
    ("Add Task", "Thêm việc", "添加任务", "Ajouter"),
    ("Edit Task", "Sửa việc", "编辑任务", "Modifier"),
    ("Task title", "Tên việc", "任务名称", "Titre de la tâche"),
    ("Starts", "Bắt đầu", "开始", "Début"),
    ("Save", "Lưu", "保存", "Enregistrer"),
    ("Cancel", "Huỷ", "取消", "Annuler"),
    ("No tasks.", "Chưa có việc nào.", "暂无任务。", "Aucune tâche."),
    (" min", " phút", " 分钟", " min"),
    ("for", "trong", "时长", "pendant"),
    // Prayer
    ("Prayer times", "Giờ cầu nguyện", "祷告时间", "Heures de prière"),
    ("are off", "đang tắt", "已关闭", "désactivées"),
    ("Turn on", "Bật", "开启", "Activer"),
    ("Next", "Tiếp theo", "下一次", "Prochaine"),
    ("Next, tomorrow", "Tiếp theo, ngày mai", "下一次，明天", "Prochaine, demain"),
    ("in {}", "sau {}", "{}后", "dans {}"),
    // Today
    ("No new notifications", "Không có thông báo mới", "没有新通知", "Aucune nouvelle notification"),
    ("New ones show up here", "Thông báo mới sẽ hiện ở đây", "新通知会显示在这里", "Les nouvelles s'afficheront ici"),
    ("Notifications are off in Settings", "Thông báo đang tắt trong Cài đặt", "通知已在设置中关闭", "Les notifications sont désactivées dans Réglages"),
    // Times
    ("Just now", "Vừa xong", "刚刚", "À l'instant"),
    ("{} min ago", "{} phút trước", "{} 分钟前", "il y a {} min"),
    ("{} h ago", "{} giờ trước", "{} 小时前", "il y a {} h"),
    ("{} d ago", "{} ngày trước", "{} 天前", "il y a {} j"),
    ("{} min", "{} phút", "{} 分钟", "{} min"),
    ("{} h {} min", "{} giờ {} phút", "{} 小时 {} 分钟", "{} h {} min"),
    // Settings
    ("General", "Chung", "通用", "Général"),
    ("Language", "Ngôn ngữ", "语言", "Langue"),
    ("Start with Windows", "Khởi động cùng Windows", "开机启动", "Lancer avec Windows"),
    ("Size", "Kích thước", "大小", "Taille"),
    ("Small", "Nhỏ", "小", "Petite"),
    ("Medium", "Vừa", "中", "Moyenne"),
    ("Large", "Lớn", "大", "Grande"),
    ("Extra large", "Rất lớn", "特大", "Très grande"),
    ("Tray icon", "Biểu tượng khay", "托盘图标", "Icône de la barre"),
    ("Violet", "Tím", "紫色", "Violet"),
    ("Neon dark", "Neon tối", "暗色霓虹", "Néon sombre"),
    ("Classic light", "Cổ điển sáng", "经典浅色", "Classique clair"),
    ("Minimal notch — shrinks to a thin bar when nothing is going on", "Notch tối giản — thu thành vạch mảnh khi không có gì diễn ra", "极简刘海 — 空闲时缩成一条细线", "Encoche minimale — se réduit à une fine barre au repos"),
    ("Mascot — a little friend that lives in the notch", "Linh vật — người bạn nhỏ sống trong notch", "吉祥物 — 住在刘海里的小伙伴", "Mascotte — un petit compagnon dans l'encoche"),
    ("Open the notch on", "Mở notch khi", "打开刘海的方式", "Ouvrir l'encoche au"),
    ("Hover", "Rê chuột", "悬停", "Survol"),
    ("Click", "Bấm", "点击", "Clic"),
    ("Status ring", "Vòng trạng thái", "状态环", "Anneau d'état"),
    ("Tasks done today", "Việc xong hôm nay", "今日完成任务", "Tâches faites aujourd'hui"),
    ("No ring", "Không hiện", "不显示", "Aucun"),
    ("Reminder sounds — chimes for Pomodoro, To-do and Water", "Âm báo — chuông cho Pomodoro, Việc cần làm và Nước", "提醒音 — 番茄钟、待办和喝水的提示音", "Sons de rappel — carillons pour Pomodoro, À faire et Eau"),
    ("To-do reminders", "Nhắc việc", "待办提醒", "Rappels des tâches"),
    ("When a task starts", "Khi việc bắt đầu", "任务开始时", "Au début d'une tâche"),
    ("When time is up", "Khi hết giờ", "时间到时", "À la fin du temps"),
    ("\"Later\" waits", "\"Để sau\" chờ", "\"稍后\"等待", "« Plus tard » attend"),
    ("Water reminders", "Nhắc uống nước", "喝水提醒", "Rappels d'eau"),
    ("\"Snooze\" waits", "\"Để sau\" chờ", "\"稍后\"等待", "« Plus tard » attend"),
    ("5 min", "5 phút", "5 分钟", "5 min"),
    ("10 min", "10 phút", "10 分钟", "10 min"),
    ("15 min", "15 phút", "15 分钟", "15 min"),
    ("Show prayer times — Kemenag RI method", "Hiện giờ cầu nguyện — theo cách tính Kemenag RI", "显示祷告时间 — Kemenag RI 算法", "Afficher les heures de prière — méthode Kemenag RI"),
    ("City", "Thành phố", "城市", "Ville"),
    ("Remind me at prayer time", "Nhắc khi đến giờ", "到时提醒我", "Me rappeler à l'heure"),
    ("Also 10 minutes before", "Nhắc thêm trước 10 phút", "提前 10 分钟也提醒", "Aussi 10 minutes avant"),
    ("Pause media at prayer time", "Tạm dừng nhạc khi đến giờ", "到时暂停媒体播放", "Mettre en pause à l'heure de la prière"),
    ("Adjust — minutes added to each time", "Điều chỉnh — số phút cộng vào mỗi giờ", "调整 — 每个时间加上的分钟数", "Ajuster — minutes ajoutées à chaque heure"),
    ("Start with an empty Shelf", "Mở với Kệ trống", "启动时清空收纳架", "Démarrer avec une étagère vide"),
    ("Clipboard history — text you copy, ready to paste again", "Lịch sử clipboard — chữ bạn chép, sẵn sàng dán lại", "剪贴板历史 — 复制过的文字，随时再粘贴", "Historique du presse-papiers — le texte copié, prêt à recoller"),
    ("Keep up to", "Giữ tối đa", "最多保留", "Garder jusqu'à"),
    (" items", " mục", " 项", " éléments"),
    ("Remember after restart", "Nhớ sau khi khởi động lại", "重启后保留", "Garder après redémarrage"),
    ("Popups", "Cửa sổ bật lên", "弹出提示", "Fenêtres"),
    ("Wi-Fi", "Wi-Fi", "Wi-Fi", "Wi-Fi"),
    ("Bluetooth headset", "Tai nghe Bluetooth", "蓝牙耳机", "Casque Bluetooth"),
    ("Hide in full screen — games, videos and slides that fill the screen", "Ẩn khi toàn màn hình — game, video và trình chiếu", "全屏时隐藏 — 游戏、视频和幻灯片", "Masquer en plein écran — jeux, vidéos et présentations"),
    ("Notifications", "Thông báo", "通知", "Notifications"),
    ("Show notifications — new Windows notifications appear here (after restart)", "Hiện thông báo — thông báo mới của Windows hiện ở đây (sau khi khởi động lại)", "显示通知 — Windows 新通知显示在这里（重启后生效）", "Afficher les notifications — les nouvelles notifications Windows apparaissent ici (après redémarrage)"),
    ("Show the message text", "Hiện nội dung tin nhắn", "显示消息内容", "Afficher le texte du message"),
    ("Today page — recent notifications and your week", "Trang Hôm nay — thông báo gần đây và tuần của bạn", "今日页面 — 最近的通知和本周安排", "Page Aujourd'hui — notifications récentes et votre semaine"),
    ("Show lyrics — from LRCLIB, a community lyrics database (after restart)", "Hiện lời bài hát — từ LRCLIB, kho lời do cộng đồng đóng góp (sau khi khởi động lại)", "显示歌词 — 来自社区歌词库 LRCLIB（重启后生效）", "Afficher les paroles — depuis LRCLIB, base communautaire (après redémarrage)"),
    ("Left widget", "Tiện ích trái", "左侧小组件", "Widget de gauche"),
    ("Right widget", "Tiện ích phải", "右侧小组件", "Widget de droite"),
    ("World clock, first city", "Giờ thế giới, thành phố 1", "世界时钟，城市一", "Horloge mondiale, ville 1"),
    ("World clock, second city", "Giờ thế giới, thành phố 2", "世界时钟，城市二", "Horloge mondiale, ville 2"),
    ("Countdown to", "Đếm ngược tới", "倒计时至", "Compte à rebours jusqu'à"),
    ("Week starts on", "Tuần bắt đầu từ", "每周开始于", "La semaine commence le"),
    ("Sunday", "Chủ nhật", "星期日", "Dimanche"),
    ("Monday", "Thứ hai", "星期一", "Lundi"),
    ("Close", "Đóng", "关闭", "Fermer"),
    ("Show the date and time — the clock in the notch and on each page", "Hiện ngày giờ — đồng hồ trên notch và ở mỗi trang", "显示日期和时间 — 刘海和每页上的时钟", "Afficher la date et l'heure — l'horloge dans l'encoche et sur chaque page"),
    ("Pages", "Các trang", "页面", "Pages"),
    ("Move up", "Chuyển lên", "上移", "Monter"),
    ("Move down", "Chuyển xuống", "下移", "Descendre"),
    ("Ask Nusa", "Hỏi Nusa", "问 Nusa", "Demander à Nusa"),
    ("Ask anything, or a sum like 12*3=", "Hỏi bất cứ điều gì, hoặc phép tính như 12*3=", "随便问，或输入算式如 12*3=", "Posez une question, ou un calcul comme 12*3="),
    ("Ask Nusa needs a Gemini key — add one in Settings", "Hỏi Nusa cần khóa Gemini — thêm trong Cài đặt", "问 Nusa 需要 Gemini 密钥 — 请在设置中添加", "Demander à Nusa nécessite une clé Gemini — ajoutez-la dans Réglages"),
    ("Enter to ask · Tab to translate · Esc to close", "Enter để hỏi · Tab để dịch · Esc để đóng", "回车提问 · Tab 翻译 · Esc 关闭", "Entrée pour demander · Tab pour traduire · Échap pour fermer"),
    ("Go on by itself — the next break or focus starts without a click", "Tự chạy tiếp — hết giờ thì nghỉ hoặc tập trung tự bắt đầu, không cần bấm", "自动继续 — 下一段休息或专注无需点击即开始", "Enchaîner — la pause ou le focus suivant démarre sans clic"),
    ("Pomodoro paused", "Pomodoro đã tạm dừng", "番茄钟已暂停", "Pomodoro en pause"),
    ("The PC was asleep — press Resume to go on", "Máy vừa ngủ — bấm Tiếp tục để chạy tiếp", "电脑刚才处于睡眠 — 点击继续即可接着计时", "Le PC était en veille — appuyez sur Reprendre"),
    ("Focused today: {}", "Hôm nay đã tập trung: {}", "今天已专注：{}", "Concentré aujourd'hui : {}"),
    ("Quiet focus — notifications wait until focus ends", "Tập trung yên tĩnh — thông báo chờ đến khi hết giờ tập trung", "专注免打扰 — 通知等到专注结束再显示", "Focus au calme — les notifications attendent la fin du focus"),
    ("Start the 4 rounds over", "Bắt đầu lại từ vòng 1", "从第 1 轮重新开始", "Recommencer les 4 tours"),
    ("Start this part over", "Chạy lại phần này từ đầu", "从头开始这一段", "Recommencer cette partie"),
    ("Skip to the next part", "Bỏ qua, sang phần kế tiếp", "跳到下一段", "Passer à la partie suivante"),
    ("Switch player ({} open)", "Đổi trình phát (đang mở {})", "切换播放器（已打开 {} 个）", "Changer de lecteur ({} ouverts)"),
    ("Welcome to My Notch", "Chào mừng đến với My Notch", "欢迎使用 My Notch", "Bienvenue dans My Notch"),
    ("Hover to open · right-click for Settings", "Rê chuột để mở · click phải để cài đặt", "悬停打开 · 右键进入设置", "Survolez pour ouvrir · clic droit : réglages"),
    ("Click to open · right-click for Settings", "Click để mở · click phải để cài đặt", "点击打开 · 右键进入设置", "Cliquez pour ouvrir · clic droit : réglages"),
    ("Shortcut used by another app", "Phím tắt đang bị app khác dùng", "快捷键已被其他应用占用", "Raccourci pris par une autre app"),
    ("Taken: {}", "Bị chiếm: {}", "被占用：{}", "Pris : {}"),
    ("Shortcuts — Ctrl+Alt+Space asks Nusa, Ctrl+Alt+T copies text from the screen", "Phím tắt — Ctrl+Alt+Space để hỏi Nusa, Ctrl+Alt+T để lấy chữ trên màn hình", "快捷键 — Ctrl+Alt+Space 问 Nusa，Ctrl+Alt+T 复制屏幕上的文字", "Raccourcis — Ctrl+Alt+Espace pour demander à Nusa, Ctrl+Alt+T pour copier le texte de l'écran"),
    ("Ready", "Sẵn sàng", "可用", "Prêts"),
    ("Thinking…", "Đang nghĩ…", "思考中…", "Réflexion…"),
    ("Gemini turned the key down — check it in Settings", "Gemini từ chối khóa — kiểm tra lại trong Cài đặt", "Gemini 拒绝了密钥 — 请在设置中检查", "Gemini a refusé la clé — vérifiez-la dans Réglages"),
    ("Too many questions just now — try again in a minute", "Hỏi nhiều quá — thử lại sau một phút", "提问过于频繁 — 请一分钟后再试", "Trop de questions pour l'instant — réessayez dans une minute"),
    ("Couldn't reach Gemini — check the internet", "Không kết nối được Gemini — kiểm tra mạng", "无法连接 Gemini — 请检查网络", "Gemini injoignable — vérifiez la connexion"),
    ("Gemini gave no answer to that", "Gemini không trả lời câu này", "Gemini 没有回答这个问题", "Gemini n'a pas répondu à cela"),
    ("Paste here", "Dán vào đây", "粘贴到这里", "Collez ici"),
    ("Gemini key — free at aistudio.google.com/apikey; kept sealed on this PC", "Khóa Gemini — lấy miễn phí ở aistudio.google.com/apikey; được mã hóa trên máy này", "Gemini 密钥 — 在 aistudio.google.com/apikey 免费获取；在本机加密保存", "Clé Gemini — gratuite sur aistudio.google.com/apikey ; chiffrée sur ce PC"),
    ("Google Calendar", "Google Lịch", "Google 日历", "Google Agenda"),
    ("Secret iCal address — Google Calendar › Settings › your calendar › Integrate calendar", "Địa chỉ iCal bí mật — Google Lịch › Cài đặt › lịch của bạn › Tích hợp lịch", "iCal 私密地址 — Google 日历 › 设置 › 你的日历 › 集成日历", "Adresse iCal secrète — Google Agenda › Paramètres › votre agenda › Intégrer l'agenda"),
    ("Today's events show in To-do and remind you when they start", "Sự kiện hôm nay hiện trong Danh sách và nhắc khi bắt đầu", "今天的日程显示在待办中，开始时提醒", "Les événements du jour s'affichent dans la liste et vous préviennent au début"),
    ("The address should start with https://", "Địa chỉ phải bắt đầu bằng https://", "地址应以 https:// 开头", "L'adresse doit commencer par https://"),
    ("{} events today", "{} sự kiện hôm nay", "今天 {} 个日程", "{} événements aujourd'hui"),
    ("Couldn't read the calendar — check the address and the internet", "Không đọc được lịch — kiểm tra địa chỉ và mạng", "无法读取日历 — 请检查地址和网络", "Impossible de lire l'agenda — vérifiez l'adresse et la connexion"),
    ("Reading the calendar…", "Đang đọc lịch…", "正在读取日历…", "Lecture de l'agenda…"),
    ("Calendar", "Lịch", "日历", "Agenda"),
    ("All day", "Cả ngày", "全天", "Toute la journée"),
    ("(No title)", "(Không tiêu đề)", "（无标题）", "(Sans titre)"),
    ("Quit", "Thoát", "退出", "Quitter"),
    ("Exit", "Thoát", "退出", "Quitter"),
    ("Show prayer times — worked out on this PC, for any city", "Hiện giờ cầu nguyện — tính ngay trên máy, cho mọi thành phố", "显示祷告时间 — 在本机计算，适用于任何城市", "Afficher les heures de prière — calculées sur ce PC, pour toute ville"),
    ("My location", "Vị trí của tôi", "我的位置", "Ma position"),
    ("Find my location", "Tìm vị trí của tôi", "查找我的位置", "Trouver ma position"),
    ("Finding where you are…", "Đang tìm vị trí…", "正在查找位置…", "Recherche de votre position…"),
    ("Couldn't find your location", "Không tìm được vị trí", "无法找到你的位置", "Position introuvable"),
    ("Check Windows Settings › Privacy › Location, or pick a city in Settings", "Kiểm tra Cài đặt Windows › Quyền riêng tư › Vị trí, hoặc chọn thành phố trong Cài đặt", "请检查 Windows 设置 › 隐私 › 位置，或在设置中选择城市", "Vérifiez Paramètres Windows › Confidentialité › Position, ou choisissez une ville"),
    ("No location yet", "Chưa có vị trí", "还没有位置", "Pas encore de position"),
    ("Play it again 10 minutes later", "Phát lại sau 10 phút", "10 分钟后继续播放", "Reprendre 10 minutes plus tard"),
    ("Asr time", "Giờ Asr", "晡礼时间", "Heure de l'Asr"),
    ("As the method does", "Theo phương pháp", "按所选方法", "Selon la méthode"),
    ("Shafi'i (shadow ×1)", "Shafi'i (bóng ×1)", "沙斐仪派（影长×1）", "Chafiite (ombre ×1)"),
    ("Hanafi (shadow ×2)", "Hanafi (bóng ×2)", "哈乃斐派（影长×2）", "Hanafite (ombre ×2)"),
    ("Imsak {} · Sunrise {}", "Imsak {} · Mặt trời mọc {}", "伊姆萨克 {} · 日出 {}", "Imsak {} · Lever du soleil {}"),
    // The Hijri months (prayer::HIJRI_MONTHS).
    ("Muharram", "Muharram", "穆哈兰月", "Mouharram"),
    ("Safar", "Safar", "色法尔月", "Safar"),
    ("Rabi' al-Awwal", "Rabi' al-Awwal", "赖比尔·敖外鲁月", "Rabia al-awal"),
    ("Rabi' al-Thani", "Rabi' al-Thani", "赖比尔·阿色尼月", "Rabia ath-thani"),
    ("Jumada al-Ula", "Jumada al-Ula", "主马达·敖外鲁月", "Joumada al-oula"),
    ("Jumada al-Akhirah", "Jumada al-Akhirah", "主马达·阿色尼月", "Joumada ath-thania"),
    ("Rajab", "Rajab", "赖哲卜月", "Rajab"),
    ("Sha'ban", "Sha'ban", "舍尔邦月", "Chaabane"),
    ("Shawwal", "Shawwal", "闪瓦鲁月", "Chawwal"),
    ("Dhu al-Qi'dah", "Dhu al-Qi'dah", "都尔喀尔德月", "Dhou al-qi'da"),
    ("Dhu al-Hijjah", "Dhu al-Hijjah", "都尔黑哲月", "Dhou al-hijja"),
    // The five prayers (prayer::NAMES).
    ("Fajr", "Fajr", "晨礼", "Fajr"),
    ("Dhuhr", "Dhuhr", "晌礼", "Dhohr"),
    ("Asr", "Asr", "晡礼", "Asr"),
    ("Maghrib", "Maghrib", "昏礼", "Maghrib"),
    ("Isha", "Isha", "宵礼", "Icha"),
    ("Find it in Settings › Prayer times", "Tìm trong Cài đặt › Giờ cầu nguyện", "请在 设置 › 礼拜时间 中查找", "À trouver dans Réglages › Heures de prière"),
    ("Prayer times need to know where you are", "Giờ cầu nguyện cần biết bạn đang ở đâu", "礼拜时间需要知道你在哪里", "Les heures de prière ont besoin de votre position"),
    ("Location is off — turn it on in Windows Settings › Privacy › Location", "Vị trí đang tắt — bật trong Cài đặt Windows › Quyền riêng tư › Vị trí", "位置已关闭 — 请在 Windows 设置 › 隐私 › 位置 中开启", "La localisation est désactivée — activez-la dans Paramètres Windows › Confidentialité › Localisation"),
    ("Couldn't find your location — check Windows Settings › Privacy › Location, or pick a city", "Không tìm được vị trí — kiểm tra Cài đặt Windows › Quyền riêng tư › Vị trí, hoặc chọn thành phố", "无法获取位置 — 请检查 Windows 设置 › 隐私 › 位置，或选择城市", "Position introuvable — vérifiez Paramètres Windows › Confidentialité › Localisation, ou choisissez une ville"),
    ("Couldn't copy", "Không chép được", "无法复制", "Copie impossible"),
    ("Weather", "Thời tiết", "天气", "Météo"),
    ("WEATHER", "THỜI TIẾT", "天气", "MÉTÉO"),
    ("Weather for", "Thời tiết ở", "天气地点", "Météo pour"),
    ("Getting the weather…", "Đang lấy thời tiết…", "正在获取天气…", "Récupération de la météo…"),
    ("Find my location in Settings", "Tìm vị trí trong Cài đặt", "请在设置中查找我的位置", "Trouvez ma position dans Réglages"),
    ("Clear sky", "Trời quang", "晴", "Ciel dégagé"),
    ("Fair", "Quang đãng", "晴间少云", "Beau temps"),
    ("Partly cloudy", "Có mây", "多云间晴", "Partiellement nuageux"),
    ("Cloudy", "Nhiều mây", "多云", "Nuageux"),
    ("Fog", "Sương mù", "雾", "Brouillard"),
    ("Rain", "Mưa", "雨", "Pluie"),
    ("Showers", "Mưa rào", "阵雨", "Averses"),
    ("Snow", "Tuyết", "雪", "Neige"),
    ("Sleet", "Mưa tuyết", "雨夹雪", "Neige fondue"),
    ("Thunder", "Dông", "雷雨", "Orage"),
    ("Getting today's rates…", "Đang lấy tỷ giá hôm nay…", "正在获取今日汇率…", "Récupération des taux du jour…"),
    ("Rates need the internet — try again in a minute", "Cần mạng để lấy tỷ giá — thử lại sau một phút", "汇率需要联网 — 请一分钟后再试", "Les taux nécessitent Internet — réessayez dans une minute"),
    ("Unknown currency code: {} or {}", "Không rõ mã tiền tệ: {} hoặc {}", "未知货币代码：{} 或 {}", "Code de devise inconnu : {} ou {}"),
    ("Another app is using the clipboard — try again", "Ứng dụng khác đang dùng clipboard — thử lại nhé", "其他应用正在使用剪贴板 — 请重试", "Une autre appli utilise le presse-papiers — réessayez"),
    ("Not found yet", "Chưa tìm", "尚未查找", "Pas encore trouvée"),
    ("Usual for the city", "Theo thành phố", "按城市惯例", "Selon la ville"),
    ("Method", "Cách tính", "计算方法", "Méthode"),
    ("Text copied", "Đã chép chữ", "已复制文字", "Texte copié"),
    ("{} characters, ready to paste", "{} ký tự, sẵn sàng để dán", "{} 个字符，可直接粘贴", "{} caractères, prêts à coller"),
    ("No text found", "Không thấy chữ", "未找到文字", "Aucun texte trouvé"),
    ("Try a larger area", "Thử chọn vùng lớn hơn", "请框选更大的区域", "Essayez une zone plus grande"),
    ("Add an OCR language in Windows Settings › Language", "Thêm ngôn ngữ OCR trong Cài đặt Windows › Ngôn ngữ", "请在 Windows 设置 › 语言 中添加 OCR 语言", "Ajoutez une langue OCR dans Paramètres Windows › Langue"),
    ("Mascot", "Linh vật", "吉祥物", "Mascotte"),
    ("Minimal notch", "Notch tối giản", "极简刘海", "Encoche minimale"),
    ("About My Notch", "Giới thiệu My Notch", "关于 My Notch", "À propos de My Notch"),
    ("Version {}", "Phiên bản {}", "版本 {}", "Version {}"),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_phrase_is_listed_once_with_matching_gaps() {
        for (i, row) in TABLE.iter().enumerate() {
            assert!(!TABLE[..i].iter().any(|r| r.0 == row.0), "{:?} is listed twice", row.0);
            let gaps = row.0.matches("{}").count();
            for tr in [row.1, row.2, row.3] {
                assert!(!tr.is_empty(), "{:?} has an empty translation", row.0);
                assert_eq!(tr.matches("{}").count(), gaps, "{:?} -> {:?} has other gaps", row.0, tr);
                // Settings rows split "Name — hint" on this separator.
                assert_eq!(tr.contains(" — "), row.0.contains(" — "), "{:?} -> {:?} lost its name/hint split", row.0, tr);
            }
        }
    }

    #[test]
    fn every_phrase_the_app_asks_for_is_in_the_table() {
        for src in [include_str!("main.rs"), include_str!("tray.rs")] {
            for start in ["t(\"", "tr(\""] {
                for (at, _) in src.match_indices(start) {
                    if src[..at].chars().next_back().is_some_and(|c| c.is_alphanumeric() || c == '_') {
                        continue; // part of a longer name, e.g. "print("
                    }
                    let rest = &src[at + start.len()..];
                    let mut end = 0;
                    while let Some(i) = rest[end..].find('"') {
                        end += i;
                        if !rest[..end].ends_with('\\') {
                            break;
                        }
                        end += 1;
                    }
                    let key = rest[..end].replace("\\\"", "\"");
                    assert!(TABLE.iter().any(|r| r.0 == key), "{key:?} is asked for but not in the table");
                }
            }
        }
        // Names looked up through a list rather than written out.
        for key in crate::prayer::NAMES.iter().chain(&crate::prayer::HIJRI_MONTHS).chain(&crate::EVENTS).chain(&crate::WIDGETS).copied() {
            assert!(TABLE.iter().any(|r| r.0 == key), "{key:?} is asked for but not in the table");
        }
    }

    #[test]
    fn falls_back_to_english_and_fills_gaps() {
        let _lock = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        set(VI);
        assert_eq!(t("Settings"), "Cài đặt");
        assert_eq!(t("Not a listed phrase"), "Not a listed phrase");
        assert_eq!(tr("{} min ago", &[&5]), "5 phút trước");
        set(FR);
        assert_eq!(tr("{} h {} min", &[&2, &5]), "2 h 5 min");
        let d = chrono::NaiveDate::from_ymd_opt(2026, 11, 20).unwrap().and_hms_opt(9, 0, 0).unwrap();
        assert_eq!(date(d), "ven. 20 nov. 2026");
        set(ZH);
        assert_eq!(date(d), "2026年11月20日 周五");
        set(EN);
        assert_eq!(date(d), "Fri 20 Nov 2026");
    }
}
