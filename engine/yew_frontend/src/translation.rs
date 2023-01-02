// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::frontend::use_ctw;
use core_protocol::id::LanguageId::*;
use core_protocol::id::{GameId, LanguageId, PeriodId};
use yew::hook;

/// Only works in function component.
#[hook]
pub fn use_translation() -> LanguageId {
    use_ctw().setting_cache.language
}

/// Declare static translations.
#[macro_export]
macro_rules! s {
    ($name: ident) => {
        fn $name(self) -> &'static str;
    };
    ($name: ident, $value: expr) => {
        fn $name(self) -> &'static str {
            $value
        }
    };
}

#[macro_export]
macro_rules! sd {
    ($name: ident, $doc: literal) => {
        #[doc = $doc]
        fn $name(self) -> &'static str;
    };
}

/// Re-use static translations.
#[macro_export]
macro_rules! sl {
    ($name: ident, $link: ident) => {
        fn $name(self) -> &'static str {
            self.$link()
        }
    };
}

// All functions take self not &self so that it is possible to construct function pointers that
// implement PartialEq (used to customize translations).
pub trait Translation: Sized {
    sd!(label, "The name of the language, in the language.");

    // Chat.
    sd!(chat_label, "Generic chat label.");
    sd!(chat_radio_label, "Alternate chat label for combat games.");
    s!(chat_send_message_hint);
    s!(chat_send_team_message_hint);
    s!(chat_send_message_placeholder);
    s!(chat_report_label);
    s!(chat_mute_label);

    // Live-board/leaderboard.
    sd!(liveboard_label, "Header for live leaderboard.");
    fn leaderboard_label(self, period_id: PeriodId) -> &'static str {
        match period_id {
            PeriodId::AllTime => self.leaderboard_all_time_label(),
            PeriodId::Daily => self.leaderboard_daily_label(),
            PeriodId::Weekly => self.leaderboard_weekly_label(),
        }
    }
    s!(leaderboard_all_time_label);
    s!(leaderboard_daily_label);
    s!(leaderboard_weekly_label);

    // Teams.
    s!(team_label);
    s!(team_accept_hint);
    s!(team_accept_full_hint);
    s!(team_create_hint);
    s!(team_deny_hint);
    s!(team_kick_hint);
    s!(team_leave_hint);
    s!(team_name_placeholder);
    s!(team_request_hint);

    // Players online.
    fn online(self, players: u32) -> String;

    // Upgrading.
    s!(upgrade_label);
    fn upgrade_to_label(self, upgrade: &str) -> String;
    fn downgrade_to_label(self, downgrade: &str) -> String;
    fn upgrade_to_level_label(self, level: u32) -> String;
    fn upgrade_to_level_progress(self, percent: u8, level: u32) -> String;

    // Respawning.
    fn respawn_as_level_label(self, level: u32) -> String;

    // Zoom.
    s!(zoom_in_hint);
    s!(zoom_out_hint);

    // Splash screen.
    s!(splash_screen_play_label);
    s!(splash_screen_alias_placeholder);

    // Invitation.
    s!(invitation_hint);
    s!(invitation_label);
    s!(invitation_copied_label);

    // Connection lost.
    s!(connection_losing_message);
    s!(connection_lost_message);

    // Alert
    s!(alert_dismiss);

    // Score.
    s!(point);
    s!(points);
    fn score(self, score: u32) -> String {
        // Good enough for simple plural vs. singular dichotomy, but can be overridden if needed.
        let suffix = match score {
            1 => self.point(),
            _ => self.points(),
        };
        format!("{} {}", score, suffix)
    }

    // About.
    s!(about_hint);
    fn about_title(self, game_id: GameId) -> String;

    // Help.
    s!(help_hint);
    fn help_title(self, game_id: GameId) -> String;
    s!(learn_more_label);

    // Settings.
    s!(settings_hint);
    s!(settings_title);
    s!(settings_language_hint);
    s!(settings_volume_hint);

    // Changelog.
    s!(changelog_hint);
    fn changelog_title(self, game_id: GameId) -> String;

    // Privacy.
    s!(privacy_hint);
    fn privacy_title(self, game_id: GameId) -> String;

    // Terms.
    s!(terms_hint);
    fn terms_title(self, game_id: GameId) -> String;
}

impl Translation for LanguageId {
    fn label(self) -> &'static str {
        match self {
            Bork => "Bork, bork, bork!",
            German => "Deutsch",
            English => "English",
            Spanish => "Español",
            French => "Français",
            Italian => "Italiano",
            Arabic => "العربية",
            Japanese => "日本語",
            Russian => "Русский",
            Vietnamese => "Tiếng Việt",
            SimplifiedChinese => "简体中文",
            Hindi => "हिन्दी",
        }
    }

    fn chat_label(self) -> &'static str {
        match self {
            Bork => "Messagebork",
            German => "Chat",
            English => "Chat",
            Spanish => "Charlar",
            French => "Discuter",
            Italian => "Chiacchierata",
            Arabic => "العربية",
            Japanese => "チャット",
            Russian => "Чат",
            Vietnamese => "Trò chuyện",
            SimplifiedChinese => "聊天",
            Hindi => "चैट",
        }
    }

    fn chat_radio_label(self) -> &'static str {
        match self {
            Bork => self.chat_label(),
            German => "Radio",
            English => "Radio",
            Spanish => "Radio",
            French => "Radio",
            Italian => "Radio",
            Arabic => "ثَرْثَرَ",
            Japanese => "無線",
            Russian => "Радио",
            Vietnamese => "Đài",
            SimplifiedChinese => "聊天",
            Hindi => "रेडियो",
        }
    }

    fn chat_send_message_hint(self) -> &'static str {
        match self {
            Bork => "Press Enter to bork",
            German => "Drücke Enter um eine Nachricht zu senden",
            English => "Press Enter to send",
            Spanish => "Presione Enter para enviar",
            French => "Appuyez sur Entrée pour envoyer",
            Italian => "Premi Invio per inviare",
            Arabic => "اضغط على Enter للإرسال",
            Japanese => "Enterキーを押して送信します",
            Russian => "Нажмите Enter, чтобы отправить",
            Vietnamese => "Nhấn Enter để gửi",
            SimplifiedChinese => "按回车发送",
            Hindi => "भेजने के लिए एंटर दबाएं",
        }
    }

    fn chat_send_team_message_hint(self) -> &'static str {
        match self {
            Bork => "Messagebork",
            German => "Drücke Enter um eine Nachricht an alle zu schicken oder Shift+Enter um eine Nachricht an deine Teammitglieder zu schicken.",
            English => "Press Enter to send, or Shift+Enter to send to team only",
            Spanish => "Presiona Enter para enviar, o Shift + Enter para enviar solo al equipo",
            French => "Appuyez sur Entrée pour envoyer ou sur Maj+Entrée pour envoyer à l'équipe uniquement",
            Italian => "Premi Invio per inviare, oppure Shift+Invio per inviare solo al team",
            Arabic => "اضغط على Enter للإرسال، أو Shift+Enter للإرسال إلى الفريق فقط",
            Japanese => "Enterキーを押して送信するか、Shift + Enterキーを押してチームのみに送信します",
            Russian => "Нажмите Enter, чтобы отправить, или Shift + Enter, чтобы отправить только группе.",
            Vietnamese => "Nhấn Enter để gửi hoặc Shift + Enter để chỉ gửi cho nhóm",
            SimplifiedChinese => "按 Enter 发送，或 Shift+Enter 仅发送给团队",
            Hindi => "Press Enter to send, or Shift+Enter to send to team only"
        }
    }

    fn chat_send_message_placeholder(self) -> &'static str {
        match self {
            Bork => "Bork",
            German => "Nachricht",
            English => "Message",
            Spanish => "Mensaje",
            French => "Message",
            Italian => "Messaggio",
            Arabic => "رسالة",
            Japanese => "メッセージ",
            Russian => "Сообщение",
            Vietnamese => "Thông điệp",
            SimplifiedChinese => "信息",
            Hindi => "संदेश",
        }
    }

    fn chat_mute_label(self) -> &'static str {
        match self {
            German => "Stummschalten",
            English | Bork => "Mute",
            Spanish => "Silenciarlo",
            French => "Mettez",
            Italian => "Mutili",
            Arabic => "كتم الصوت",
            Japanese => "ミュートする",
            Russian => "Отключить",
            Vietnamese => "Tắt tiếng",
            SimplifiedChinese => "沉默的",
            Hindi => "म्यूट",
        }
    }

    fn chat_report_label(self) -> &'static str {
        match self {
            German => "Melden",
            English | Bork => "Report",
            Spanish => "Reportalo",
            French => "Signalez",
            Italian => "Denunciarlo",
            Arabic => "بلغ عنه",
            Japanese => "彼を報告する",
            Russian => "Сообщить",
            Vietnamese => "Báo cáo",
            SimplifiedChinese => "举报",
            Hindi => "रिपोर्ट",
        }
    }

    fn liveboard_label(self) -> &'static str {
        match self {
            Bork => "Leaderbork",
            German => "Bestenliste",
            English => "Leaderboard",
            Spanish => "Tabla",
            French => "Classement",
            Italian => "Classifica",
            Arabic => "المتصدرين",
            Japanese => "リーダーボード",
            Russian => "Таблица лидеров",
            Vietnamese => "Bảng xếp hạng",
            SimplifiedChinese => "排行榜",
            Hindi => "लीडरबोर्ड",
        }
    }

    fn leaderboard_all_time_label(self) -> &'static str {
        match self {
            Bork => "All-time Leaderbork",
            German => "Bestenliste (Jemals)",
            English => "All-time Leaderboard",
            Spanish => "Tabla de todos los tiempos",
            French => "Classement de tous les temps",
            Italian => "Classifica di tutti i tempi",
            Arabic => "لوحة المتصدرين في كل الأوقات",
            Japanese => "史上最高のリーダーボード",
            Russian => "Таблица лидеров за все время",
            Vietnamese => "Bảng xếp hạng mọi thời đại",
            SimplifiedChinese => "历史排行榜",
            Hindi => "ऑल-टाइम लीडरबोर्ड",
        }
    }

    fn leaderboard_daily_label(self) -> &'static str {
        match self {
            Bork => "Daily Leaderbork",
            German => "Bestenliste (Täglich)",
            English => "Daily Leaderboard",
            Spanish => "Tabla diaria",
            French => "Classement quotidien",
            Italian => "Classifica Giornaliera",
            Arabic => "لوحة المتصدرين اليومية",
            Japanese => "デイリーリーダーボード",
            Russian => "Ежедневная таблица лидеров",
            Vietnamese => "Bảng xếp hạng hàng ngày",
            SimplifiedChinese => "每日排行榜",
            Hindi => "दैनिक लीडरबोर्क",
        }
    }

    fn leaderboard_weekly_label(self) -> &'static str {
        match self {
            Bork => "Weekly Leaderbork",
            German => "Bestenliste (Wöchentlich)",
            English => "Weekly Leaderboard",
            Spanish => "Tabla semanal",
            French => "Classement hebdomadaire",
            Italian => "Classifica Settimanale",
            Arabic => "المتصدرين الأسبوعية",
            Japanese => "ウィークリーリーダーボード",
            Russian => "Еженедельная таблица лидеров",
            Vietnamese => "Bảng xếp hạng hàng tuần",
            SimplifiedChinese => "每周排行榜",
            Hindi => "साप्ताहिक लीडरबोर्ड",
        }
    }

    fn team_label(self) -> &'static str {
        match self {
            Bork => "Borks",
            German => "Team",
            English => "Team",
            Spanish => "Equipo",
            French => "Équipe",
            Italian => "Squadra",
            Arabic => "فريق",
            Japanese => "チーム",
            Russian => "Команда",
            Vietnamese => "Đội",
            SimplifiedChinese => "团队",
            Hindi => "टीम",
        }
    }

    fn team_accept_hint(self) -> &'static str {
        match self {
            Bork => "Bork",
            German => "Annehmen",
            English => "Accept",
            Spanish => "Aceptar",
            French => "J'accepte",
            Italian => "Accetta",
            Arabic => "تقبل",
            Japanese => "承認",
            Russian => "Принимать",
            Vietnamese => "Chấp nhận",
            SimplifiedChinese => "接受",
            Hindi => "स्वीकार",
        }
    }

    fn team_accept_full_hint(self) -> &'static str {
        match self {
            Bork => "Team borked",
            German => "Team voll",
            English => "Team full",
            Spanish => "El equipo esta lleno",
            French => "Equipe au complet",
            Italian => "Squadra piena",
            Arabic => "فريق كامل",
            Japanese => "チームがいっぱい",
            Russian => "Команда заполнена",
            Vietnamese => "Đội đầy đủ",
            SimplifiedChinese => "团队满员",
            Hindi => "पूरी टीम",
        }
    }

    fn team_create_hint(self) -> &'static str {
        match self {
            Bork => "Bork",
            German => "Erstellen",
            English => "Create",
            Spanish => "Crear",
            French => "Créer",
            Italian => "Crea",
            Arabic => "خلق",
            Japanese => "作成",
            Russian => "Создавать",
            Vietnamese => "Tạo ra",
            SimplifiedChinese => "创造",
            Hindi => "बनाना",
        }
    }

    fn team_deny_hint(self) -> &'static str {
        match self {
            Bork => "Unbork",
            German => "Ablehnen",
            English => "Deny",
            Spanish => "Negar",
            French => "Refuser",
            Italian => "Rifiuta",
            Arabic => "أنكر",
            Japanese => "拒否",
            Russian => "Отрицать",
            Vietnamese => "Từ chối",
            SimplifiedChinese => "拒绝",
            Hindi => "मना",
        }
    }

    fn team_kick_hint(self) -> &'static str {
        match self {
            Bork => "Unbork",
            German => "Rauswerfen",
            English => "Kick",
            Spanish => "Retirar",
            French => "Coup",
            Italian => "Kick",
            Arabic => "رفس",
            Japanese => "追放",
            Russian => "Удар",
            Vietnamese => "Trục xuất",
            SimplifiedChinese => "踢出",
            Hindi => "लात",
        }
    }

    fn team_leave_hint(self) -> &'static str {
        match self {
            Bork => "Unbork",
            German => "Verlassen",
            English => "Leave",
            Spanish => "Dejar",
            French => "Quitter",
            Italian => "Abbandona",
            Arabic => "ترك",
            Japanese => "離れる",
            Russian => "Покинуть",
            Vietnamese => "Rời bỏ",
            SimplifiedChinese => "离开",
            Hindi => "छुट्टी",
        }
    }

    fn team_name_placeholder(self) -> &'static str {
        match self {
            Bork => "Name of borks",
            German => "Teamname",
            English => "Team name",
            Spanish => "Nombre del equipo",
            French => "Nom de l'équipe",
            Italian => "Nome della squadra",
            Arabic => "اسم الفريق",
            Japanese => "チームの名前",
            Russian => "Название команды",
            Vietnamese => "Tên nhóm",
            SimplifiedChinese => "队名",
            Hindi => "टीम का नाम",
        }
    }

    fn team_request_hint(self) -> &'static str {
        match self {
            Bork => "Bork",
            German => "Anfragen",
            English => "Request Join",
            Spanish => "Solicitar unirse",
            French => "Demande d'adhésion",
            Italian => "Chiedi di unirti",
            Arabic => "طلب الانضمام",
            Japanese => "参加をリクエストする",
            Russian => "Запрос",
            Vietnamese => "thỉnh cầu",
            SimplifiedChinese => "请求加入",
            Hindi => "शामिल होने का अनुरोध",
        }
    }

    fn online(self, players: u32) -> String {
        match self {
            Bork => format!("{players} borks"),
            German => format!("{players} Spieler"),
            English => format!("{players} online"),
            Spanish => format!("{players} en línea"),
            French => format!("{players} en ligne"),
            Italian => format!("{players} online"),
            Arabic => format!("على الانترنت {players}"),
            Japanese => format!("{players}オンライン"),
            Russian => format!("{players} онлайн"),
            Vietnamese => format!("{players} trực tuyến"),
            SimplifiedChinese => format!("{players}玩家"),
            Hindi => format!("{players} ऑनलाइन"),
        }
    }

    fn upgrade_label(self) -> &'static str {
        match self {
            Bork => "Bork",
            German => "Verbessern",
            English => "Upgrade",
            Spanish => "Actualiza",
            French => "Passer",
            Italian => "Sali",
            Arabic => "ترقيه",
            Japanese => "アドバンス",
            Russian => "модернизировать",
            Vietnamese => "Trèo lên",
            SimplifiedChinese => "提升",
            Hindi => "उन्नत करना",
        }
    }

    fn upgrade_to_label(self, upgrade: &str) -> String {
        match self {
            Bork => format!("Bork to {upgrade}"),
            German => format!("Auf {upgrade} upgraden"),
            English => format!("Upgrade to {upgrade}"),
            Spanish => format!("Actualiza al {upgrade}"),
            French => format!("Passer au {upgrade}"),
            Italian => format!("Sali al {upgrade}"),
            Arabic => format!("الترقية إلى {upgrade}"),
            Japanese => format!("{upgrade} にアップグレード"),
            Russian => format!("Обновить до {upgrade}"),
            Vietnamese => format!("Nâng cấp lên {upgrade}"),
            SimplifiedChinese => format!("升级到 {upgrade}"),
            Hindi => format!("{upgrade} में अपग्रेड करें"),
        }
    }

    fn downgrade_to_label(self, downgrade: &str) -> String {
        match self {
            Bork => format!("Debork to {downgrade}"),
            German => format!("Downgrade auf {downgrade}"),
            English => format!("Downgrade to {downgrade}"),
            Spanish => format!("Rebajar a {downgrade}"),
            French => format!("Rétrograder à {downgrade}"),
            Italian => format!("Esegui il downgrade a {downgrade}"),
            Arabic => format!("الرجوع إلى {downgrade}"),
            Japanese => format!("{downgrade} にダウングレード"),
            Russian => format!("Перейти на {downgrade}"),
            Vietnamese => format!("Hạ cấp xuống {downgrade}"),
            SimplifiedChinese => format!("降级为 {downgrade}"),
            Hindi => format!("डाउनग्रेड करके {downgrade} करें"),
        }
    }

    fn upgrade_to_level_label(self, level: u32) -> String {
        match self {
            Bork => format!("Bork to level {level}"),
            German => format!("Auf Level {level} upgraden"),
            English => format!("Upgrade to level {level}"),
            Spanish => format!("Actualiza al nivel {level}"),
            French => format!("Passer au niveau {level}"),
            Italian => format!("Sali al livello {level}"),
            Arabic => format!("الترقية إلى المستوى {level}"),
            Japanese => format!("レベル{level}にアップグレードする"),
            Russian => format!("Перейти на уровень {level}"),
            Vietnamese => format!("Nâng cấp lên cấp {level}"),
            SimplifiedChinese => format!("升级到级别 {level}"),
            Hindi => format!("स्तर {level} . में अपग्रेड करें"),
        }
    }

    fn upgrade_to_level_progress(self, percent: u8, level: u32) -> String {
        match self {
            German => format!("{percent} % bis Stufe {level}"),
            English => format!("{percent}% to level {level}"),
            Bork => format!("{percent}% to bork {level}"),
            Spanish => format!("{percent}% al nivel {level}"),
            French => format!("{percent}% au niveau {level}"),
            Italian => format!("{percent}% al livello {level}"),
            Arabic => format!("{level}٪ إلى مستوى {percent}"),
            Japanese => format!("レベル{level}まで{percent}%"),
            Russian => format!("{percent}% до {level} уровня"),
            Vietnamese => format!("{percent}% lên cấp {level}"),
            SimplifiedChinese => format!("{percent}% 到 {level} 级"),
            Hindi => format!("{percent}% से स्तर {level}"),
        }
    }

    fn respawn_as_level_label(self, level: u32) -> String {
        match self {
            Bork => format!("Rebork as level {level}"),
            German => format!("Als Level {level} respawnen"),
            English => format!("Respawn as level {level}"),
            Spanish => format!("Reaparecer como nivel {level}"),
            French => format!("Réapparaître au niveau {level}"),
            Italian => format!("Respawn come livello {level}"),
            Arabic => format!("إعادة التفريخ كمستوى {level}"),
            Japanese => format!("レベル{level}としてリスポーン"),
            Russian => format!("Возрождение на уровне {level}"),
            Vietnamese => format!("Được tạo lại dưới dạng cấp {level}"),
            SimplifiedChinese => format!("重生为级别 {level}"),
            Hindi => format!("स्तर {level} . के रूप में प्रतिक्रिया"),
        }
    }

    fn zoom_in_hint(self) -> &'static str {
        match self {
            Bork => "Bork In",
            German => "Reinzoomen",
            English => "Zoom In",
            Spanish => "Acercarse",
            French => "Agrandir",
            Italian => "Ingrandire",
            Arabic => "التكبير",
            Japanese => "ズームイン",
            Russian => "Увеличить",
            Vietnamese => "Phóng to",
            SimplifiedChinese => "放大",
            Hindi => "ज़ूम इन",
        }
    }

    fn zoom_out_hint(self) -> &'static str {
        match self {
            Bork => "Bork Out",
            German => "Rauszoomen",
            English => "Zoom Out",
            Spanish => "Disminuir el zoom",
            French => "Dézoomer",
            Italian => "Rimpicciolire",
            Arabic => "التصغير",
            Japanese => "ズームアウトする",
            Russian => "Уменьшить",
            Vietnamese => "Thu nhỏ",
            SimplifiedChinese => "缩小",
            Hindi => "ज़ूम आउट",
        }
    }

    fn splash_screen_play_label(self) -> &'static str {
        match self {
            Bork => "Bork",
            German => "Spielen",
            English => "Play",
            Spanish => "Comienzo",
            French => "Démarrer",
            Italian => "Gioca",
            Arabic => "لعب",
            Japanese => "演奏する",
            Russian => "Начинать",
            Vietnamese => "Chơi",
            SimplifiedChinese => "开始",
            Hindi => "खेलते",
        }
    }

    fn splash_screen_alias_placeholder(self) -> &'static str {
        match self {
            Bork => "Bork",
            German => "Spitzname",
            English => "Nickname",
            Spanish => "Apodo",
            French => "Surnom",
            Italian => "Nickname",
            Arabic => "لقب",
            Japanese => "ニックネーム",
            Russian => "прозвище",
            Vietnamese => "Biệt danh",
            SimplifiedChinese => "昵称",
            Hindi => "उपनाम",
        }
    }

    sl!(invitation_hint, invitation_label);

    fn invitation_label(self) -> &'static str {
        match self {
            Bork => "Copy Bork",
            German => "Link kopieren",
            English => "Copy Invite",
            Spanish => "Copiar invitación",
            French => "Copier l'invitation",
            Italian => "Copia Invito",
            Arabic => "نسخ الدعوة",
            Japanese => "招待をコピー",
            Russian => "Копировать приглашение",
            Vietnamese => "Sao chép lời mời",
            SimplifiedChinese => "复制邀请",
            Hindi => "कॉपी आमंत्रण",
        }
    }

    fn invitation_copied_label(self) -> &'static str {
        match self {
            Bork => "Borked!",
            German => "Kopiert!",
            English => "Copied!",
            Spanish => "¡Copiada!",
            French => "Copié !",
            Italian => "Copiato!",
            Arabic => "نسخ!",
            Japanese => "コピーしました！",
            Russian => "Скопировано!",
            Vietnamese => "Đã sao chép!",
            SimplifiedChinese => "复制！",
            Hindi => "कॉपी किया गया!",
        }
    }

    fn connection_lost_message(self) -> &'static str {
        match self {
            Bork => "Your connection was borked. Try again later!",
            German => "Verbindung zum Server verloren. Versuchen Sie es später noch einmal!",
            English => "Lost connection to server. Try again later!",
            Spanish => "Se perdió la conexión con el servidor. ¡Inténtalo de nuevo más tarde!",
            French => "Connexion perdue au serveur. Réessayez plus tard!",
            Italian => "Connessione persa al server. Riprovare più tardi!",
            Arabic => "انقطع الاتصال بالخادم. حاول مرة أخرى في وقت لاحق!",
            Japanese => "サーバーへの接続が失われました。あとでもう一度試してみてください！",
            Russian => "Потеряно соединение с сервером. Попробуйте позже!",
            Vietnamese => "Mất kết nối với máy chủ. Thử lại sau!",
            SimplifiedChinese => "失去与服务器的连接。稍后再试！",
            Hindi => "सर्वर से कनेक्शन टूट गया। बाद में पुन: प्रयास!",
        }
    }

    fn connection_losing_message(self) -> &'static str {
        match self {
            Bork => "Your connection was borked. Reborking now...",
            German => "Verbindung unterbrochen, versucht, die Verbindung wiederherzustellen...",
            English => "Connection lost, attempting to reconnect...",
            Spanish => "Conexión perdida, intentando volver a conectar...",
            French => "Connexion perdue, tentative de reconnexion...",
            Italian => "Connessione persa, tentativo di riconnessione in corso...",
            Arabic => "انقطع الاتصال أثناء محاولة إعادة الاتصال ...",
            Japanese => "接続が失われました。再接続を試みています...",
            Russian => "Соединение потеряно, попытка переподключения...",
            Vietnamese => "Mất kết nối, đang cố kết nối lại ...",
            SimplifiedChinese => "连接丢失，正在尝试重新连接...",
            Hindi => "कनेक्शन टूट गया, फिर से कनेक्ट करने का प्रयास...",
        }
    }

    /*
    fn connection_lost_message(self) -> &'static str {
        match self {
            Bork => "Your connection was borked. Try again later!",
            German => "Die Schlacht ist vorbei. Du kannst es in wenigen Momenten erneut versuchen.",
            English => "The battle is over. Try starting again shortly.",
            Spanish => "La batalla ha terminado. Intente comenzar de nuevo en breve.",
            French => "La bataille est terminée. Essayez de recommencer sous peu.",
            Italian => "La battaglia è finita. Ritenta a breve.",
            Arabic => "المعركة انتهت. حاول البدء مرة أخرى قريبا.",
            Japanese => "戦いは終わった。すぐにやり直してください。",
            Russian => "Битва окончена. Попробуйте начать снова в ближайшее время.",
            Vietnamese => "Trận chiến kết thúc. Hãy thử bắt đầu lại trong thời gian ngắn.",
            SimplifiedChinese => "战斗结束了。稍后重新开始尝试。",
            Hindi => "लड़ाई खत्म हो गई है। शीघ्र ही पुन: प्रारंभ करने का प्रयास करें।",
        }
    }
     */

    fn alert_dismiss(self) -> &'static str {
        match self {
            Bork => "Bork",
            German => "Zurückweisen",
            English => "Dismiss",
            Spanish => "Despedir",
            French => "Rejeter",
            Italian => "Congedare",
            Arabic => "رفض",
            Japanese => "解散",
            Russian => "Увольнять",
            Vietnamese => "Bỏ qua",
            SimplifiedChinese => "解雇",
            Hindi => "नकार देना",
        }
    }

    fn point(self) -> &'static str {
        match self {
            Bork => "bork",
            German => "Punkt",
            English => "point",
            Spanish => "punto",
            French => "point",
            Italian => "punto",
            Arabic => "نقطة",
            Japanese => "点数",
            Russian => "балл",
            Vietnamese => "điểm",
            SimplifiedChinese => "分",
            Hindi => "अंक",
        }
    }

    fn points(self) -> &'static str {
        match self {
            Bork => "borks",
            German => "Punkte",
            English => "points",
            Spanish => "puntos",
            French => "points",
            Italian => "punti",
            Arabic => "النقاط",
            Japanese => "点数",
            Russian => "баллов",
            Vietnamese => "điểm",
            SimplifiedChinese => "分",
            Hindi => "अंक",
        }
    }

    fn about_hint(self) -> &'static str {
        match self {
            Bork => "Bork?!",
            German => "Über",
            English => "About",
            Spanish => "Acerca",
            French => "À propos",
            Italian => "Riferimenti",
            Arabic => "عن",
            Japanese => "だいたい",
            Russian => "О",
            Vietnamese => "Về",
            SimplifiedChinese => "关于",
            Hindi => "बारे में",
        }
    }

    fn about_title(self, game_id: GameId) -> String {
        let name = game_id.name();
        match self {
            Bork => format!("{name}?!"),
            German => format!("Über {name}"),
            English => format!("About {name}"),
            Spanish => format!("Sobre {name}"),
            French => format!("À propos de {name}"),
            Italian => format!("Riguardo {name}"),
            Arabic => format!("حوالي {name}"),
            Japanese => format!("{name}について"),
            Russian => format!("О {name}"),
            Vietnamese => format!("Về {name}"),
            SimplifiedChinese => format!("关于 {name}"),
            Hindi => format!("{name} . के बारे में"),
        }
    }

    fn help_hint(self) -> &'static str {
        match self {
            Bork => "Bork?",
            German => "Hilfe",
            English => "Help",
            Spanish => "Ayuda",
            French => "Aide",
            Italian => "Aiuto",
            Arabic => "تعليمات",
            Japanese => "ヘルプ",
            Russian => "Помощь",
            Vietnamese => "Cứu giúp",
            SimplifiedChinese => "帮助",
            Hindi => "मदद करना",
        }
    }

    fn help_title(self, game_id: GameId) -> String {
        let name = game_id.name();
        match self {
            Bork => format!("{name}?"),
            German => format!("{name} Hilfe"),
            English => format!("{name} Help Guide"),
            Spanish => format!("Guía de ayuda de {name}"),
            French => format!("Guide d'aide {name}"),
            Italian => format!("{name} Guida di aiuto"),
            Arabic => format!("{name} دليل المساعدة"),
            Japanese => format!("{name}ヘルプガイド"),
            Russian => format!("Справочное руководство {name}"),
            Vietnamese => format!("Hướng dẫn trợ giúp {name}"),
            SimplifiedChinese => format!("{name} 帮助指南"),
            Hindi => format!("{name} सहायता"),
        }
    }

    fn learn_more_label(self) -> &'static str {
        match self {
            Bork => "Bork?",
            German => "Mehr erfahren",
            English => "Learn more",
            Spanish => "Aprende más",
            French => "Apprendre encore plus",
            Italian => "Scopri di più",
            Arabic => "يتعلم أكثر",
            Japanese => "もっと詳しく知る",
            Russian => "Учить больше",
            Vietnamese => "Tìm hiểu thêm",
            SimplifiedChinese => "学到更多",
            Hindi => "और अधिक जानें",
        }
    }

    sl!(settings_hint, settings_title);

    fn settings_title(self) -> &'static str {
        match self {
            Bork => "Borkonfiguration",
            German => "Einstellungen",
            English => "Settings",
            Spanish => "Ajustes",
            French => "Paramètres",
            Italian => "Impostazioni",
            Arabic => "اعدادات",
            Japanese => "設定",
            Russian => "Настройки",
            Vietnamese => "Cài đặt",
            SimplifiedChinese => "竖",
            Hindi => "समायोजन",
        }
    }

    fn settings_language_hint(self) -> &'static str {
        match self {
            Bork => "Bork, bork, bork?!",
            German => "Sprache",
            English => "Language",
            Spanish => "El lenguaje",
            French => "Langue",
            Italian => "Lingua",
            Arabic => "اللغة",
            Japanese => "言語",
            Russian => "Язык",
            Vietnamese => "Ngôn ngữ",
            SimplifiedChinese => "语",
            Hindi => "भाषा",
        }
    }

    fn settings_volume_hint(self) -> &'static str {
        match self {
            Bork => "bork <-> BORK",
            German => "Lautstärke",
            English => "Volume",
            Spanish => "Volumen",
            French => "Le volume",
            Italian => "Volume",
            Arabic => "حجم",
            Japanese => "音量",
            Russian => "громкость",
            Vietnamese => "Âm lượng",
            SimplifiedChinese => "音量",
            Hindi => "मात्रा",
        }
    }

    fn changelog_hint(self) -> &'static str {
        match self {
            Bork => "Borklog",
            German => "Änderungsprotokoll",
            English => "Changelog",
            Spanish => "Actualizaciones",
            French => "Mises à jour",
            Italian => "Aggiornamenti",
            Arabic => "سجل التغيير",
            Japanese => "変更ログ",
            Russian => "Обновления",
            Vietnamese => "Cập nhật",
            SimplifiedChinese => "变更日志",
            Hindi => "चेंजलॉग",
        }
    }

    fn changelog_title(self, game_id: GameId) -> String {
        let name = game_id.name();
        match self {
            Bork => format!("{} Borklog", name),
            German => format!("{} Updates", name),
            English => format!("{} Changelog", name),
            Spanish => format!("Registro de cambios de {}", name),
            French => format!("Journal des modifications de {}", name),
            Italian => format!("{} Registro delle modifiche", name),
            Arabic => format!("{} سجل التغيير", name),
            Japanese => format!("{}変更ログ", name),
            Russian => format!("История изменений {}", name),
            Vietnamese => format!("{} Nhật ký thay đổi", name),
            SimplifiedChinese => format!("{} 更新日志", name),
            Hindi => format!("{name} चेंजलॉग"),
        }
    }

    fn privacy_hint(self) -> &'static str {
        match self {
            German => "Datenschutz",
            English | Bork => "Privacy",
            Spanish => "Intimidad",
            French => "Confidentialité",
            Italian => "Privacy",
            Arabic => "الخصوصيه",
            Japanese => "プライバシー",
            Russian => "секретность",
            Vietnamese => "Sự riêng tư",
            SimplifiedChinese => "隐私",
            Hindi => "गोपनीयता",
        }
    }

    fn privacy_title(self, game_id: GameId) -> String {
        let name = game_id.name();
        match self {
            Bork => format!("{} Privacy Bork", name),
            German => format!("{} Datenschutz", name),
            English => format!("{} Privacy Policy", name),
            Spanish => format!("Política de privacidad de {}", name),
            French => format!("Politique de confidentialité de {}", name),
            Italian => format!("{} Informativa sulla Privacy", name),
            Arabic => format!("{} سياسة الخصوصية", name),
            Japanese => format!("{}プライバシーポリシー", name),
            Russian => format!("Политика конфиденциальности {}", name),
            Vietnamese => format!("Chính sách quyền riêng tư của {}", name),
            SimplifiedChinese => format!("{} 隐私政策", name),
            Hindi => format!("{name} गोपनीयता नीति"),
        }
    }

    fn terms_hint(self) -> &'static str {
        match self {
            Bork => "Terms",
            German => "AGB",
            English => "Terms",
            Spanish => "Condiciones",
            French => "Termes",
            Italian => "Termini",
            Arabic => "حيث",
            Japanese => "条項",
            Russian => "Условия",
            Vietnamese => "Điều kiện",
            SimplifiedChinese => "条款",
            Hindi => "शर्तें",
        }
    }

    fn terms_title(self, game_id: GameId) -> String {
        let name = game_id.name();
        match self {
            Bork => format!("{} Terms of Bork", name),
            German => format!("{} AGB", name),
            English => format!("{} Terms of Service", name),
            Spanish => format!("Condiciones de servicio de {}", name),
            French => format!("Conditions d'utilisation de {}", name),
            Italian => format!("{} Termini del Servizio", name),
            Arabic => format!("{} شروط الخدمة", name),
            Japanese => format!("{}利用規約", name),
            Russian => format!("Условия использования {}", name),
            Vietnamese => format!("{} Điều khoản dịch vụ", name),
            SimplifiedChinese => format!("{} 服务条款", name),
            Hindi => format!("{name} सेवा की शर्तें"),
        }
    }
}
