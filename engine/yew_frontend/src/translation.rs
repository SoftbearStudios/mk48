use crate::frontend::Ctw;
use core_protocol::id::LanguageId::*;
use core_protocol::id::{GameId, LanguageId};

/// Only works in function component.
pub fn use_translation() -> LanguageId {
    yew::use_context::<Ctw>().unwrap().setting_cache.language_id
}

/// Alias of [`use_translation`] to be more concise.
pub fn t() -> LanguageId {
    use_translation()
}

/// Declare static translations.
#[macro_export]
macro_rules! s {
    ($name: ident) => {
        fn $name(&self) -> &'static str;
    };
    ($name: ident, $value: expr) => {
        fn $name(&self) -> &'static str {
            $value
        }
    };
}

#[macro_export]
macro_rules! sd {
    ($name: ident, $doc: literal) => {
        #[doc = $doc]
        fn $name(&self) -> &'static str;
    };
}

/// Re-use static translations.
#[macro_export]
macro_rules! sl {
    ($name: ident, $link: ident) => {
        fn $name(&self) -> &'static str {
            self.$link()
        }
    };
}

pub trait Translation {
    sd!(label, "The name of the language, in the language.");

    // Chat.
    sd!(chat_label, "Generic chat label.");
    sd!(chat_radio_label, "Alternate chat label for combat games.");
    s!(chat_send_message_hint);
    s!(chat_send_team_message_hint);
    s!(chat_send_message_placeholder);

    // Live-board/leaderboard.
    sd!(liveboard_label, "Header for live leaderboard.");
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
    fn online(&self, players: u32) -> String;

    // Upgrading.
    s!(upgrade_label);
    fn upgrade_to_level_label(&self, level: u32) -> String;
    fn upgrade_to_level_progress(&self, percent: u8, level: u32) -> String;

    // Respawning.
    fn respawn_as(&self, level: u32) -> String;

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
    s!(connection_lost_message);

    // Score.
    s!(point);
    s!(points);
    fn score(&self, score: u32) -> String {
        // Good enough for simple plural vs. singular dichotomy, but can be overridden if needed.
        let suffix = match score {
            1 => self.point(),
            _ => self.points(),
        };
        format!("{} {}", score, suffix)
    }

    // About.
    s!(about_hint);
    fn about_title(&self, game_id: GameId) -> String;

    // Help.
    s!(help_hint);
    fn help_title(&self, game_id: GameId) -> String;

    // Settings.
    s!(settings_hint);
    s!(settings_title);
    s!(settings_language_hint);
    s!(settings_volume_hint);

    // Changelog.
    s!(changelog_hint);
    fn changelog_title(&self, game_id: GameId) -> String;

    // Privacy.
    s!(privacy_hint);
    fn privacy_title(&self, game_id: GameId) -> String;

    // Terms.
    s!(terms_hint);
    fn terms_title(&self, game_id: GameId) -> String;
}

impl Translation for LanguageId {
    fn label(&self) -> &'static str {
        match self {
            Bork => "Bork, bork, bork!",
            German => "Deutsch",
            English => "English",
            Spanish => "Español",
            French => "Français",
            Italian => "Italiano",
            Arabic => "العربية",
            Japanese => "日本",
            Russian => "русский",
            Vietnamese => "Tiếng Việt",
            SimplifiedChinese => "简体中文",
        }
    }

    fn chat_label(&self) -> &'static str {
        match self {
            Bork => "Messagebork",
            German => "Plaudern",
            English => "Chat",
            Spanish => "Charlar",
            French => "Discuter",
            Italian => "Chiacchierata",
            Arabic => "العربية",
            Japanese => "チャット",
            Russian => "Чат",
            Vietnamese => "Trò chuyện",
            SimplifiedChinese => "聊天",
        }
    }

    fn chat_radio_label(&self) -> &'static str {
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
        }
    }

    fn chat_send_message_hint(&self) -> &'static str {
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
        }
    }

    fn chat_send_team_message_hint(&self) -> &'static str {
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
        }
    }

    fn chat_send_message_placeholder(&self) -> &'static str {
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
        }
    }

    fn liveboard_label(&self) -> &'static str {
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
        }
    }

    fn leaderboard_all_time_label(&self) -> &'static str {
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
        }
    }

    fn leaderboard_daily_label(&self) -> &'static str {
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
        }
    }

    fn leaderboard_weekly_label(&self) -> &'static str {
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
        }
    }

    fn team_label(&self) -> &'static str {
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
        }
    }

    fn team_accept_hint(&self) -> &'static str {
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
        }
    }

    fn team_accept_full_hint(&self) -> &'static str {
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
        }
    }

    fn team_create_hint(&self) -> &'static str {
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
        }
    }

    fn team_deny_hint(&self) -> &'static str {
        match self {
            Bork => "Unbork",
            German => "Anlehnen",
            English => "Deny",
            Spanish => "Negar",
            French => "Refuser",
            Italian => "Rifiuta",
            Arabic => "أنكر",
            Japanese => "拒否",
            Russian => "Отрицать",
            Vietnamese => "Từ chối",
            SimplifiedChinese => "拒绝",
        }
    }

    fn team_kick_hint(&self) -> &'static str {
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
        }
    }

    fn team_leave_hint(&self) -> &'static str {
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
        }
    }

    fn team_name_placeholder(&self) -> &'static str {
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
        }
    }

    fn team_request_hint(&self) -> &'static str {
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
        }
    }

    fn online(&self, players: u32) -> String {
        match self {
            Bork => format!("{} borks", players),
            German => format!("{} Spieler", players),
            English => format!("{} online", players),
            Spanish => format!("{} en línea", players),
            French => format!("{} en ligne", players),
            Italian => format!("{} online", players),
            Arabic => format!("على الانترنت {}", players),
            Japanese => format!("{}オンライン", players),
            Russian => format!("{} онлайн", players),
            Vietnamese => format!("{} trực tuyến", players),
            SimplifiedChinese => format!("{}玩家", players),
        }
    }

    fn upgrade_label(&self) -> &'static str {
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
        }
    }

    fn upgrade_to_level_label(&self, level: u32) -> String {
        match self {
            Bork => format!("Bork to level {}", level),
            German => format!("Auf Level {} upgraden", level),
            English => format!("Upgrade to level {}", level),
            Spanish => format!("Actualiza al nivel {}", level),
            French => format!("Passer au niveau {}", level),
            Italian => format!("Sali al livello {}", level),
            Arabic => format!("الترقية إلى المستوى {}", level),
            Japanese => format!("レベル{}にアップグレードする", level),
            Russian => format!("Перейти на уровень {}", level),
            Vietnamese => format!("Nâng cấp lên cấp {}", level),
            SimplifiedChinese => format!("升级到级别 {}", level),
        }
    }

    fn upgrade_to_level_progress(&self, percent: u8, level: u32) -> String {
        match self {
            German => format!("{} % bis Stufe {}", percent, level),
            English => format!("{}% to level {}", percent, level),
            Bork => format!("{}% to bork {}", percent, level),
            Spanish => format!("{}% al nivel {}", percent, level),
            French => format!("{}% au niveau {}", percent, level),
            Italian => format!("{}% al livello {}", percent, level),
            Arabic => format!("{}٪ إلى مستوى {}", percent, level),
            Japanese => format!("レベル{}まで{}%", level, percent),
            Russian => format!("{}% до {} уровня", percent, level),
            Vietnamese => format!("{}% lên cấp {}", percent, level),
            SimplifiedChinese => format!("{}% 到 {} 级", percent, level),
        }
    }

    fn respawn_as(&self, level: u32) -> String {
        match self {
            Bork => format!("Rebork as level {}", level),
            German => format!("Respawnen Sie als Level {}", level),
            English => format!("Respawn as level {}", level),
            Spanish => format!("Reaparecer como nivel {}", level),
            French => format!("Réapparaître au niveau {}", level),
            Italian => format!("Respawn come livello {}", level),
            Arabic => format!("إعادة التفريخ كمستوى {}", level),
            Japanese => format!("レベル{}としてリスポーン", level),
            Russian => format!("Возрождение на уровне {}", level),
            Vietnamese => format!("Được tạo lại dưới dạng cấp {}", level),
            SimplifiedChinese => format!("重生为级别 {}", level),
        }
    }

    fn zoom_in_hint(&self) -> &'static str {
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
        }
    }

    fn zoom_out_hint(&self) -> &'static str {
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
        }
    }

    fn splash_screen_play_label(&self) -> &'static str {
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
        }
    }

    fn splash_screen_alias_placeholder(&self) -> &'static str {
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
        }
    }

    sl!(invitation_hint, invitation_label);

    fn invitation_label(&self) -> &'static str {
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
        }
    }

    fn invitation_copied_label(&self) -> &'static str {
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
        }
    }

    fn connection_lost_message(&self) -> &'static str {
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
        }
    }

    fn point(&self) -> &'static str {
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
        }
    }

    fn points(&self) -> &'static str {
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
        }
    }

    fn about_hint(&self) -> &'static str {
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
        }
    }

    fn about_title(&self, game_id: GameId) -> String {
        let name = game_id.name();
        match self {
            Bork => format!("{}?!", name),
            German => format!("Über {}", name),
            English => format!("About {}", name),
            Spanish => format!("Sobre {}", name),
            French => format!("À propos de {}", name),
            Italian => format!("Riguardo {}", name),
            Arabic => format!("حوالي {}", name),
            Japanese => format!("{}について", name),
            Russian => format!("О {}", name),
            Vietnamese => format!("Về {}", name),
            SimplifiedChinese => format!("关于 {}", name),
        }
    }

    fn help_hint(&self) -> &'static str {
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
        }
    }

    fn help_title(&self, game_id: GameId) -> String {
        let name = game_id.name();
        match self {
            Bork => format!("{}?", name),
            German => format!("{} Hilfe", name),
            English => format!("{} Help Guide", name),
            Spanish => format!("Guía de ayuda de {}", name),
            French => format!("Guide d'aide {}", name),
            Italian => format!("{} Guida di aiuto", name),
            Arabic => format!("{} دليل المساعدة", name),
            Japanese => format!("{}ヘルプガイド", name),
            Russian => format!("Справочное руководство {}", name),
            Vietnamese => format!("Hướng dẫn trợ giúp {}", name),
            SimplifiedChinese => format!("{} 帮助指南", name),
        }
    }

    sl!(settings_hint, settings_title);

    fn settings_title(&self) -> &'static str {
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
        }
    }

    fn settings_language_hint(&self) -> &'static str {
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
        }
    }

    fn settings_volume_hint(&self) -> &'static str {
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
        }
    }

    fn changelog_hint(&self) -> &'static str {
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
        }
    }

    fn changelog_title(&self, game_id: GameId) -> String {
        let name = game_id.name();
        match self {
            Bork => format!("{} Borklog", name),
            German => format!("{} Updates", name),
            English => format!("{} Help Guide", name),
            Spanish => format!("Registro de cambios de {}", name),
            French => format!("Journal des modifications de {}", name),
            Italian => format!("{} Registro delle modifiche", name),
            Arabic => format!("{} سجل التغيير", name),
            Japanese => format!("{}変更ログ", name),
            Russian => format!("История изменений {}", name),
            Vietnamese => format!("{} Nhật ký thay đổi", name),
            SimplifiedChinese => format!("{} 更新日志", name),
        }
    }

    fn privacy_hint(&self) -> &'static str {
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
        }
    }

    fn privacy_title(&self, game_id: GameId) -> String {
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
        }
    }

    fn terms_hint(&self) -> &'static str {
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
        }
    }

    fn terms_title(&self, game_id: GameId) -> String {
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
        }
    }
}
