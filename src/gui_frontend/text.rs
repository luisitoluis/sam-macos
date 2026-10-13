// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Paul <abonnementspaul (at) gmail.com>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, version 3.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

use std::env;

/// Interface language. Names in the picker stay in each language's own writing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Language {
    English,
    Spanish,
    Chinese,
    Russian,
    Portuguese,
    German,
    French,
    Japanese,
    Korean,
}

impl Language {
    pub const ALL: [Language; 9] = [
        Language::English,
        Language::Spanish,
        Language::Chinese,
        Language::Russian,
        Language::Portuguese,
        Language::German,
        Language::French,
        Language::Japanese,
        Language::Korean,
    ];

    pub fn native_name(self) -> &'static str {
        match self {
            Language::English => "English",
            Language::Spanish => "Español",
            Language::Chinese => "简体中文",
            Language::Russian => "Русский",
            Language::Portuguese => "Português",
            Language::German => "Deutsch",
            Language::French => "Français",
            Language::Japanese => "日本語",
            Language::Korean => "한국어",
        }
    }

    /// Uses the process locale when it is one of the supported languages.
    pub fn from_locale() -> Self {
        let raw = env::var("LC_ALL")
            .or_else(|_| env::var("LANG"))
            .unwrap_or_default()
            .to_ascii_lowercase();
        if raw.starts_with("es") {
            Language::Spanish
        } else if raw.starts_with("zh") {
            Language::Chinese
        } else if raw.starts_with("ru") {
            Language::Russian
        } else if raw.starts_with("pt") {
            Language::Portuguese
        } else if raw.starts_with("de") {
            Language::German
        } else if raw.starts_with("fr") {
            Language::French
        } else if raw.starts_with("ja") {
            Language::Japanese
        } else if raw.starts_with("ko") {
            Language::Korean
        } else {
            Language::English
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Id {
    Library,
    Settings,
    Appearance,
    Language,
    ThemeSystem,
    ThemeDark,
    ThemeLight,
    SearchGames,
    HideJunk,
    Refresh,
    Shown,
    LoadingLibrary,
    SelectGame,
    Ready,
    Confirm,
    UnlockPrompt,
    ResetPrompt,
    Cancel,
    Continue,
    BackLibrary,
    Launch,
    TalkingToSteam,
    Achievements,
    Stats,
    Unlocked,
    UnlockAll,
    ResetAll,
    FilterAchievements,
    Protected,
    OfPlayers,
    FilterStats,
    IncrementOnly,
    Save,
    Loaded,
    AppWord,
    OpeningGame,
    OpenSteamFailed,
    WorkerStopped,
    AchievementUpdated,
    AchievementsUnlocked,
    StatsReset,
    StatSaved,
    Games,
    NotInteger,
    NotNumber,
    KindApp,
    KindMod,
    KindDemo,
    KindJunk,
}

pub fn t(lang: Language, id: Id) -> &'static str {
    match lang {
        Language::English => english(id),
        Language::Spanish => spanish(id),
        Language::Chinese => chinese(id),
        Language::Russian => russian(id),
        Language::Portuguese => portuguese(id),
        Language::German => german(id),
        Language::French => french(id),
        Language::Japanese => japanese(id),
        Language::Korean => korean(id),
    }
}

/// A status line that can be shown again after the language changes.
#[derive(Clone, Debug)]
pub enum Message {
    Phrase(Id),
    Games(usize),
    Loaded(String),
    OpenFailed(String),
    NotInteger(String),
    NotNumber(String),
    Own(String),
}

impl Message {
    pub fn text(&self, lang: Language) -> String {
        match self {
            Message::Phrase(id) => t(lang, *id).to_string(),
            Message::Games(count) => t(lang, Id::Games).replace("{n}", &count.to_string()),
            Message::Loaded(name) => t(lang, Id::Loaded).replace("{name}", name),
            Message::OpenFailed(err) => t(lang, Id::OpenSteamFailed).replace("{err}", err),
            Message::NotInteger(value) => t(lang, Id::NotInteger).replace("{value}", value),
            Message::NotNumber(value) => t(lang, Id::NotNumber).replace("{value}", value),
            Message::Own(text) => text.clone(),
        }
    }
}

pub fn shown(lang: Language, count: usize) -> String {
    t(lang, Id::Shown).replace("{n}", &count.to_string())
}

pub fn unlocked(lang: Language, done: usize, total: usize) -> String {
    t(lang, Id::Unlocked)
        .replace("{a}", &done.to_string())
        .replace("{b}", &total.to_string())
}

pub fn of_players(lang: Language, percent: f32) -> String {
    t(lang, Id::OfPlayers).replace("{p}", &format!("{percent:.1}"))
}

fn english(id: Id) -> &'static str {
    match id {
        Id::Library => "Library",
        Id::Settings => "Settings",
        Id::Appearance => "Appearance",
        Id::Language => "Language",
        Id::ThemeSystem => "System",
        Id::ThemeDark => "Dark",
        Id::ThemeLight => "Light",
        Id::SearchGames => "Search games",
        Id::HideJunk => "Hide junk",
        Id::Refresh => "Refresh",
        Id::Shown => "{n} shown",
        Id::LoadingLibrary => "Loading your Steam library…",
        Id::SelectGame => "Select a game to view achievements and stats.",
        Id::Ready => "Ready",
        Id::Confirm => "Confirm",
        Id::UnlockPrompt => "Unlock every unprotected achievement in this game?",
        Id::ResetPrompt => "Reset stats and lock achievements for this game?",
        Id::Cancel => "Cancel",
        Id::Continue => "Continue",
        Id::BackLibrary => "← Library",
        Id::Launch => "Launch",
        Id::TalkingToSteam => "Talking to Steam…",
        Id::Achievements => "Achievements",
        Id::Stats => "Stats",
        Id::Unlocked => "{a}/{b} unlocked",
        Id::UnlockAll => "Unlock all",
        Id::ResetAll => "Reset all",
        Id::FilterAchievements => "Filter achievements",
        Id::Protected => "protected",
        Id::OfPlayers => "{p}% of players",
        Id::FilterStats => "Filter stats",
        Id::IncrementOnly => "increment only",
        Id::Save => "Save",
        Id::Loaded => "Loaded {name}",
        Id::AppWord => "App",
        Id::OpeningGame => "Opening the game in Steam.",
        Id::OpenSteamFailed => "Could not open Steam: {err}",
        Id::WorkerStopped => "Steam worker stopped.",
        Id::AchievementUpdated => "Achievement updated.",
        Id::AchievementsUnlocked => "Achievements unlocked.",
        Id::StatsReset => "Stats and achievements reset.",
        Id::StatSaved => "Stat saved.",
        Id::Games => "{n} games",
        Id::NotInteger => "'{value}' is not an integer",
        Id::NotNumber => "'{value}' is not a number",
        Id::KindApp => "App",
        Id::KindMod => "Mod",
        Id::KindDemo => "Demo",
        Id::KindJunk => "Junk",
    }
}

fn spanish(id: Id) -> &'static str {
    match id {
        Id::Library => "Biblioteca",
        Id::Settings => "Ajustes",
        Id::Appearance => "Apariencia",
        Id::Language => "Idioma",
        Id::ThemeSystem => "Sistema",
        Id::ThemeDark => "Oscuro",
        Id::ThemeLight => "Claro",
        Id::SearchGames => "Buscar juegos",
        Id::HideJunk => "Ocultar basura",
        Id::Refresh => "Actualizar",
        Id::Shown => "{n} mostrados",
        Id::LoadingLibrary => "Cargando tu biblioteca de Steam…",
        Id::SelectGame => "Elige un juego para ver logros y estadísticas.",
        Id::Ready => "Listo",
        Id::Confirm => "Confirmar",
        Id::UnlockPrompt => "¿Desbloquear todos los logros no protegidos de este juego?",
        Id::ResetPrompt => "¿Reiniciar las estadísticas y bloquear los logros de este juego?",
        Id::Cancel => "Cancelar",
        Id::Continue => "Continuar",
        Id::BackLibrary => "← Biblioteca",
        Id::Launch => "Iniciar",
        Id::TalkingToSteam => "Conectando con Steam…",
        Id::Achievements => "Logros",
        Id::Stats => "Estadísticas",
        Id::Unlocked => "{a}/{b} desbloqueados",
        Id::UnlockAll => "Desbloquear todo",
        Id::ResetAll => "Reiniciar todo",
        Id::FilterAchievements => "Filtrar logros",
        Id::Protected => "protegido",
        Id::OfPlayers => "{p}% de los jugadores",
        Id::FilterStats => "Filtrar estadísticas",
        Id::IncrementOnly => "solo incrementa",
        Id::Save => "Guardar",
        Id::Loaded => "{name} cargado",
        Id::AppWord => "App",
        Id::OpeningGame => "Abriendo el juego en Steam.",
        Id::OpenSteamFailed => "No se pudo abrir Steam: {err}",
        Id::WorkerStopped => "El proceso de Steam se detuvo.",
        Id::AchievementUpdated => "Logro actualizado.",
        Id::AchievementsUnlocked => "Logros desbloqueados.",
        Id::StatsReset => "Estadísticas y logros reiniciados.",
        Id::StatSaved => "Estadística guardada.",
        Id::Games => "{n} juegos",
        Id::NotInteger => "'{value}' no es un entero",
        Id::NotNumber => "'{value}' no es un número",
        Id::KindApp => "App",
        Id::KindMod => "Mod",
        Id::KindDemo => "Demo",
        Id::KindJunk => "Basura",
    }
}

fn chinese(id: Id) -> &'static str {
    match id {
        Id::Library => "游戏库",
        Id::Settings => "设置",
        Id::Appearance => "外观",
        Id::Language => "语言",
        Id::ThemeSystem => "跟随系统",
        Id::ThemeDark => "深色",
        Id::ThemeLight => "浅色",
        Id::SearchGames => "搜索游戏",
        Id::HideJunk => "隐藏无关项",
        Id::Refresh => "刷新",
        Id::Shown => "显示 {n} 个",
        Id::LoadingLibrary => "正在加载 Steam 游戏库…",
        Id::SelectGame => "选择一个游戏以查看成就和统计数据。",
        Id::Ready => "就绪",
        Id::Confirm => "确认",
        Id::UnlockPrompt => "解锁此游戏中所有未受保护的成就？",
        Id::ResetPrompt => "重置统计数据并锁定此游戏的成就？",
        Id::Cancel => "取消",
        Id::Continue => "继续",
        Id::BackLibrary => "← 游戏库",
        Id::Launch => "启动",
        Id::TalkingToSteam => "正在连接 Steam…",
        Id::Achievements => "成就",
        Id::Stats => "统计",
        Id::Unlocked => "已解锁 {a}/{b}",
        Id::UnlockAll => "全部解锁",
        Id::ResetAll => "全部重置",
        Id::FilterAchievements => "筛选成就",
        Id::Protected => "受保护",
        Id::OfPlayers => "{p}% 的玩家",
        Id::FilterStats => "筛选统计",
        Id::IncrementOnly => "仅可增加",
        Id::Save => "保存",
        Id::Loaded => "已加载 {name}",
        Id::AppWord => "应用",
        Id::OpeningGame => "正在通过 Steam 打开游戏。",
        Id::OpenSteamFailed => "无法打开 Steam：{err}",
        Id::WorkerStopped => "Steam 工作进程已停止。",
        Id::AchievementUpdated => "成就已更新。",
        Id::AchievementsUnlocked => "成就已解锁。",
        Id::StatsReset => "统计和成就已重置。",
        Id::StatSaved => "统计已保存。",
        Id::Games => "{n} 个游戏",
        Id::NotInteger => "“{value}”不是整数",
        Id::NotNumber => "“{value}”不是数字",
        Id::KindApp => "应用",
        Id::KindMod => "Mod",
        Id::KindDemo => "试玩",
        Id::KindJunk => "无关",
    }
}

fn russian(id: Id) -> &'static str {
    match id {
        Id::Library => "Библиотека",
        Id::Settings => "Настройки",
        Id::Appearance => "Оформление",
        Id::Language => "Язык",
        Id::ThemeSystem => "Система",
        Id::ThemeDark => "Тёмная",
        Id::ThemeLight => "Светлая",
        Id::SearchGames => "Поиск игр",
        Id::HideJunk => "Скрыть лишнее",
        Id::Refresh => "Обновить",
        Id::Shown => "Показано: {n}",
        Id::LoadingLibrary => "Загрузка библиотеки Steam…",
        Id::SelectGame => "Выберите игру, чтобы увидеть достижения и статистику.",
        Id::Ready => "Готово",
        Id::Confirm => "Подтверждение",
        Id::UnlockPrompt => "Разблокировать все незащищённые достижения этой игры?",
        Id::ResetPrompt => "Сбросить статистику и заблокировать достижения этой игры?",
        Id::Cancel => "Отмена",
        Id::Continue => "Продолжить",
        Id::BackLibrary => "← Библиотека",
        Id::Launch => "Запустить",
        Id::TalkingToSteam => "Связь со Steam…",
        Id::Achievements => "Достижения",
        Id::Stats => "Статистика",
        Id::Unlocked => "Открыто {a}/{b}",
        Id::UnlockAll => "Открыть все",
        Id::ResetAll => "Сбросить всё",
        Id::FilterAchievements => "Фильтр достижений",
        Id::Protected => "защищено",
        Id::OfPlayers => "{p}% игроков",
        Id::FilterStats => "Фильтр статистики",
        Id::IncrementOnly => "только увеличение",
        Id::Save => "Сохранить",
        Id::Loaded => "Загружено: {name}",
        Id::AppWord => "Приложение",
        Id::OpeningGame => "Открываем игру в Steam.",
        Id::OpenSteamFailed => "Не удалось открыть Steam: {err}",
        Id::WorkerStopped => "Процесс Steam остановлен.",
        Id::AchievementUpdated => "Достижение обновлено.",
        Id::AchievementsUnlocked => "Достижения разблокированы.",
        Id::StatsReset => "Статистика и достижения сброшены.",
        Id::StatSaved => "Статистика сохранена.",
        Id::Games => "{n} игр",
        Id::NotInteger => "«{value}» не является целым числом",
        Id::NotNumber => "«{value}» не является числом",
        Id::KindApp => "Приложение",
        Id::KindMod => "Мод",
        Id::KindDemo => "Демо",
        Id::KindJunk => "Лишнее",
    }
}

fn portuguese(id: Id) -> &'static str {
    match id {
        Id::Library => "Biblioteca",
        Id::Settings => "Configurações",
        Id::Appearance => "Aparência",
        Id::Language => "Idioma",
        Id::ThemeSystem => "Sistema",
        Id::ThemeDark => "Escuro",
        Id::ThemeLight => "Claro",
        Id::SearchGames => "Buscar jogos",
        Id::HideJunk => "Ocultar lixo",
        Id::Refresh => "Atualizar",
        Id::Shown => "{n} exibidos",
        Id::LoadingLibrary => "Carregando sua biblioteca da Steam…",
        Id::SelectGame => "Escolha um jogo para ver conquistas e estatísticas.",
        Id::Ready => "Pronto",
        Id::Confirm => "Confirmar",
        Id::UnlockPrompt => "Desbloquear todas as conquistas desprotegidas deste jogo?",
        Id::ResetPrompt => "Redefinir as estatísticas e bloquear as conquistas deste jogo?",
        Id::Cancel => "Cancelar",
        Id::Continue => "Continuar",
        Id::BackLibrary => "← Biblioteca",
        Id::Launch => "Iniciar",
        Id::TalkingToSteam => "Falando com a Steam…",
        Id::Achievements => "Conquistas",
        Id::Stats => "Estatísticas",
        Id::Unlocked => "{a}/{b} desbloqueadas",
        Id::UnlockAll => "Desbloquear tudo",
        Id::ResetAll => "Redefinir tudo",
        Id::FilterAchievements => "Filtrar conquistas",
        Id::Protected => "protegida",
        Id::OfPlayers => "{p}% dos jogadores",
        Id::FilterStats => "Filtrar estatísticas",
        Id::IncrementOnly => "só aumenta",
        Id::Save => "Salvar",
        Id::Loaded => "{name} carregado",
        Id::AppWord => "App",
        Id::OpeningGame => "Abrindo o jogo na Steam.",
        Id::OpenSteamFailed => "Não foi possível abrir a Steam: {err}",
        Id::WorkerStopped => "O processo da Steam parou.",
        Id::AchievementUpdated => "Conquista atualizada.",
        Id::AchievementsUnlocked => "Conquistas desbloqueadas.",
        Id::StatsReset => "Estatísticas e conquistas redefinidas.",
        Id::StatSaved => "Estatística salva.",
        Id::Games => "{n} jogos",
        Id::NotInteger => "'{value}' não é um inteiro",
        Id::NotNumber => "'{value}' não é um número",
        Id::KindApp => "App",
        Id::KindMod => "Mod",
        Id::KindDemo => "Demo",
        Id::KindJunk => "Lixo",
    }
}

fn german(id: Id) -> &'static str {
    match id {
        Id::Library => "Bibliothek",
        Id::Settings => "Einstellungen",
        Id::Appearance => "Darstellung",
        Id::Language => "Sprache",
        Id::ThemeSystem => "System",
        Id::ThemeDark => "Dunkel",
        Id::ThemeLight => "Hell",
        Id::SearchGames => "Spiele suchen",
        Id::HideJunk => "Ausschuss ausblenden",
        Id::Refresh => "Aktualisieren",
        Id::Shown => "{n} angezeigt",
        Id::LoadingLibrary => "Steam-Bibliothek wird geladen…",
        Id::SelectGame => "Wähle ein Spiel, um Errungenschaften und Statistiken zu sehen.",
        Id::Ready => "Bereit",
        Id::Confirm => "Bestätigen",
        Id::UnlockPrompt => "Alle ungeschützten Errungenschaften dieses Spiels freischalten?",
        Id::ResetPrompt => "Statistiken zurücksetzen und Errungenschaften dieses Spiels sperren?",
        Id::Cancel => "Abbrechen",
        Id::Continue => "Weiter",
        Id::BackLibrary => "← Bibliothek",
        Id::Launch => "Starten",
        Id::TalkingToSteam => "Verbindung zu Steam…",
        Id::Achievements => "Errungenschaften",
        Id::Stats => "Statistiken",
        Id::Unlocked => "{a}/{b} freigeschaltet",
        Id::UnlockAll => "Alle freischalten",
        Id::ResetAll => "Alles zurücksetzen",
        Id::FilterAchievements => "Errungenschaften filtern",
        Id::Protected => "geschützt",
        Id::OfPlayers => "{p}% der Spieler",
        Id::FilterStats => "Statistiken filtern",
        Id::IncrementOnly => "nur erhöhen",
        Id::Save => "Speichern",
        Id::Loaded => "{name} geladen",
        Id::AppWord => "App",
        Id::OpeningGame => "Spiel wird in Steam geöffnet.",
        Id::OpenSteamFailed => "Steam konnte nicht geöffnet werden: {err}",
        Id::WorkerStopped => "Der Steam-Prozess wurde beendet.",
        Id::AchievementUpdated => "Errungenschaft aktualisiert.",
        Id::AchievementsUnlocked => "Errungenschaften freigeschaltet.",
        Id::StatsReset => "Statistiken und Errungenschaften zurückgesetzt.",
        Id::StatSaved => "Statistik gespeichert.",
        Id::Games => "{n} Spiele",
        Id::NotInteger => "„{value}“ ist keine ganze Zahl",
        Id::NotNumber => "„{value}“ ist keine Zahl",
        Id::KindApp => "App",
        Id::KindMod => "Mod",
        Id::KindDemo => "Demo",
        Id::KindJunk => "Ausschuss",
    }
}

fn french(id: Id) -> &'static str {
    match id {
        Id::Library => "Bibliothèque",
        Id::Settings => "Réglages",
        Id::Appearance => "Apparence",
        Id::Language => "Langue",
        Id::ThemeSystem => "Système",
        Id::ThemeDark => "Sombre",
        Id::ThemeLight => "Clair",
        Id::SearchGames => "Rechercher des jeux",
        Id::HideJunk => "Masquer le superflu",
        Id::Refresh => "Actualiser",
        Id::Shown => "{n} affichés",
        Id::LoadingLibrary => "Chargement de la bibliothèque Steam…",
        Id::SelectGame => "Choisissez un jeu pour voir les succès et les statistiques.",
        Id::Ready => "Prêt",
        Id::Confirm => "Confirmer",
        Id::UnlockPrompt => "Débloquer tous les succès non protégés de ce jeu ?",
        Id::ResetPrompt => "Réinitialiser les statistiques et verrouiller les succès de ce jeu ?",
        Id::Cancel => "Annuler",
        Id::Continue => "Continuer",
        Id::BackLibrary => "← Bibliothèque",
        Id::Launch => "Lancer",
        Id::TalkingToSteam => "Communication avec Steam…",
        Id::Achievements => "Succès",
        Id::Stats => "Statistiques",
        Id::Unlocked => "{a}/{b} débloqués",
        Id::UnlockAll => "Tout débloquer",
        Id::ResetAll => "Tout réinitialiser",
        Id::FilterAchievements => "Filtrer les succès",
        Id::Protected => "protégé",
        Id::OfPlayers => "{p} % des joueurs",
        Id::FilterStats => "Filtrer les statistiques",
        Id::IncrementOnly => "incrément seul",
        Id::Save => "Enregistrer",
        Id::Loaded => "{name} chargé",
        Id::AppWord => "App",
        Id::OpeningGame => "Ouverture du jeu dans Steam.",
        Id::OpenSteamFailed => "Impossible d'ouvrir Steam : {err}",
        Id::WorkerStopped => "Le processus Steam s'est arrêté.",
        Id::AchievementUpdated => "Succès mis à jour.",
        Id::AchievementsUnlocked => "Succès débloqués.",
        Id::StatsReset => "Statistiques et succès réinitialisés.",
        Id::StatSaved => "Statistique enregistrée.",
        Id::Games => "{n} jeux",
        Id::NotInteger => "« {value} » n'est pas un entier",
        Id::NotNumber => "« {value} » n'est pas un nombre",
        Id::KindApp => "App",
        Id::KindMod => "Mod",
        Id::KindDemo => "Démo",
        Id::KindJunk => "Superflu",
    }
}

fn japanese(id: Id) -> &'static str {
    match id {
        Id::Library => "ライブラリ",
        Id::Settings => "設定",
        Id::Appearance => "外観",
        Id::Language => "言語",
        Id::ThemeSystem => "システム",
        Id::ThemeDark => "ダーク",
        Id::ThemeLight => "ライト",
        Id::SearchGames => "ゲームを検索",
        Id::HideJunk => "不要な項目を隠す",
        Id::Refresh => "更新",
        Id::Shown => "{n} 件を表示",
        Id::LoadingLibrary => "Steamライブラリを読み込み中…",
        Id::SelectGame => "実績と統計を見るゲームを選んでください。",
        Id::Ready => "準備完了",
        Id::Confirm => "確認",
        Id::UnlockPrompt => "このゲームの保護されていない実績をすべて解除しますか？",
        Id::ResetPrompt => "統計をリセットし、このゲームの実績をロックしますか？",
        Id::Cancel => "キャンセル",
        Id::Continue => "続行",
        Id::BackLibrary => "← ライブラリ",
        Id::Launch => "起動",
        Id::TalkingToSteam => "Steamと通信中…",
        Id::Achievements => "実績",
        Id::Stats => "統計",
        Id::Unlocked => "{a}/{b} 解除済み",
        Id::UnlockAll => "すべて解除",
        Id::ResetAll => "すべてリセット",
        Id::FilterAchievements => "実績を絞り込み",
        Id::Protected => "保護",
        Id::OfPlayers => "プレイヤーの {p}%",
        Id::FilterStats => "統計を絞り込み",
        Id::IncrementOnly => "増加のみ",
        Id::Save => "保存",
        Id::Loaded => "{name} を読み込みました",
        Id::AppWord => "アプリ",
        Id::OpeningGame => "Steamでゲームを開いています。",
        Id::OpenSteamFailed => "Steamを開けませんでした: {err}",
        Id::WorkerStopped => "Steamワーカーが停止しました。",
        Id::AchievementUpdated => "実績を更新しました。",
        Id::AchievementsUnlocked => "実績を解除しました。",
        Id::StatsReset => "統計と実績をリセットしました。",
        Id::StatSaved => "統計を保存しました。",
        Id::Games => "{n} 本のゲーム",
        Id::NotInteger => "「{value}」は整数ではありません",
        Id::NotNumber => "「{value}」は数値ではありません",
        Id::KindApp => "アプリ",
        Id::KindMod => "MOD",
        Id::KindDemo => "デモ",
        Id::KindJunk => "不要",
    }
}

fn korean(id: Id) -> &'static str {
    match id {
        Id::Library => "라이브러리",
        Id::Settings => "설정",
        Id::Appearance => "모양",
        Id::Language => "언어",
        Id::ThemeSystem => "시스템",
        Id::ThemeDark => "어둡게",
        Id::ThemeLight => "밝게",
        Id::SearchGames => "게임 검색",
        Id::HideJunk => "불필요한 항목 숨기기",
        Id::Refresh => "새로고침",
        Id::Shown => "{n}개 표시",
        Id::LoadingLibrary => "Steam 라이브러리를 불러오는 중…",
        Id::SelectGame => "업적과 통계를 보려면 게임을 선택하세요.",
        Id::Ready => "준비됨",
        Id::Confirm => "확인",
        Id::UnlockPrompt => "이 게임의 보호되지 않은 업적을 모두 잠금 해제할까요?",
        Id::ResetPrompt => "통계를 초기화하고 이 게임의 업적을 잠글까요?",
        Id::Cancel => "취소",
        Id::Continue => "계속",
        Id::BackLibrary => "← 라이브러리",
        Id::Launch => "실행",
        Id::TalkingToSteam => "Steam과 통신 중…",
        Id::Achievements => "업적",
        Id::Stats => "통계",
        Id::Unlocked => "{a}/{b} 잠금 해제",
        Id::UnlockAll => "모두 잠금 해제",
        Id::ResetAll => "모두 초기화",
        Id::FilterAchievements => "업적 필터",
        Id::Protected => "보호됨",
        Id::OfPlayers => "플레이어의 {p}%",
        Id::FilterStats => "통계 필터",
        Id::IncrementOnly => "증가만 가능",
        Id::Save => "저장",
        Id::Loaded => "{name} 불러옴",
        Id::AppWord => "앱",
        Id::OpeningGame => "Steam에서 게임을 여는 중.",
        Id::OpenSteamFailed => "Steam을 열 수 없습니다: {err}",
        Id::WorkerStopped => "Steam 작업이 중지되었습니다.",
        Id::AchievementUpdated => "업적이 업데이트되었습니다.",
        Id::AchievementsUnlocked => "업적이 잠금 해제되었습니다.",
        Id::StatsReset => "통계와 업적이 초기화되었습니다.",
        Id::StatSaved => "통계가 저장되었습니다.",
        Id::Games => "게임 {n}개",
        Id::NotInteger => "'{value}'은(는) 정수가 아닙니다",
        Id::NotNumber => "'{value}'은(는) 숫자가 아닙니다",
        Id::KindApp => "앱",
        Id::KindMod => "모드",
        Id::KindDemo => "데모",
        Id::KindJunk => "불필요",
    }
}
