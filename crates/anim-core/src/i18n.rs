//! Internationalization - FR (default), EN, ZH.

use std::sync::atomic::{AtomicU8, Ordering};

static CURRENT_LANG: AtomicU8 = AtomicU8::new(0); // 0=FR, 1=EN, 2=ZH

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Lang {
    Fr = 0,
    En = 1,
    Zh = 2,
}

impl Lang {
    pub fn set(lang: Lang) {
        CURRENT_LANG.store(lang as u8, Ordering::Relaxed);
    }

    pub fn get() -> Lang {
        match CURRENT_LANG.load(Ordering::Relaxed) {
            1 => Lang::En,
            2 => Lang::Zh,
            _ => Lang::Fr,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Lang::Fr => "Français",
            Lang::En => "English",
            Lang::Zh => "中文",
        }
    }

    pub fn all() -> &'static [Lang] {
        &[Lang::Fr, Lang::En, Lang::Zh]
    }
}

/// Translate a key into the current language.
pub fn t(key: &str) -> &'static str {
    let lang = Lang::get();
    match (key, lang) {
        // === Menu ===
        ("file", Lang::Fr) => "Fichier",
        ("file", Lang::En) => "File",
        ("file", Lang::Zh) => "文件",
        ("edit", Lang::Fr) => "Édition",
        ("edit", Lang::En) => "Edit",
        ("edit", Lang::Zh) => "编辑",
        ("view", Lang::Fr) => "Affichage",
        ("view", Lang::En) => "View",
        ("view", Lang::Zh) => "视图",
        ("import", Lang::Fr) => "Importer",
        ("import", Lang::En) => "Import",
        ("import", Lang::Zh) => "导入",
        ("animation", Lang::Fr) => "Animation",
        ("animation", Lang::En) => "Animation",
        ("animation", Lang::Zh) => "动画",
        ("help", Lang::Fr) => "Aide",
        ("help", Lang::En) => "Help",
        ("help", Lang::Zh) => "帮助",
        ("language", Lang::Fr) => "Langue",
        ("language", Lang::En) => "Language",
        ("language", Lang::Zh) => "语言",

        // === Panels ===
        ("scene_hierarchy", Lang::Fr) => "Hiérarchie",
        ("scene_hierarchy", Lang::En) => "Hierarchy",
        ("scene_hierarchy", Lang::Zh) => "层级",
        ("inspector", Lang::Fr) => "Inspecteur",
        ("inspector", Lang::En) => "Inspector",
        ("inspector", Lang::Zh) => "检查器",
        ("timeline", Lang::Fr) => "Chronologie",
        ("timeline", Lang::En) => "Timeline",
        ("timeline", Lang::Zh) => "时间线",
        ("console", Lang::Fr) => "Console",
        ("console", Lang::En) => "Console",
        ("console", Lang::Zh) => "控制台",
        ("viewport", Lang::Fr) => "Vue 3D",
        ("viewport", Lang::En) => "3D Viewport",
        ("viewport", Lang::Zh) => "3D视口",

        // === Import ===
        ("import_glb", Lang::Fr) => "Importer GLB/glTF",
        ("import_glb", Lang::En) => "Import GLB/glTF",
        ("import_glb", Lang::Zh) => "导入GLB/glTF",
        ("import_bvh", Lang::Fr) => "Importer BVH",
        ("import_bvh", Lang::En) => "Import BVH",
        ("import_bvh", Lang::Zh) => "导入BVH",
        ("import_fbx", Lang::Fr) => "Importer FBX",
        ("import_fbx", Lang::En) => "Import FBX",
        ("import_fbx", Lang::Zh) => "导入FBX",

        // === Animation ===
        ("play", Lang::Fr) => "Lecture",
        ("play", Lang::En) => "Play",
        ("play", Lang::Zh) => "播放",
        ("pause", Lang::Fr) => "Pause",
        ("pause", Lang::En) => "Pause",
        ("pause", Lang::Zh) => "暂停",
        ("stop", Lang::Fr) => "Arrêt",
        ("stop", Lang::En) => "Stop",
        ("stop", Lang::Zh) => "停止",
        ("speed", Lang::Fr) => "Vitesse",
        ("speed", Lang::En) => "Speed",
        ("speed", Lang::Zh) => "速度",
        ("mirror", Lang::Fr) => "Miroir",
        ("mirror", Lang::En) => "Mirror",
        ("mirror", Lang::Zh) => "镜像",
        ("frame", Lang::Fr) => "Image",
        ("frame", Lang::En) => "Frame",
        ("frame", Lang::Zh) => "帧",
        ("loop", Lang::Fr) => "Boucle",
        ("loop", Lang::En) => "Loop",
        ("loop", Lang::Zh) => "循环",

        // === Inspector ===
        ("transform", Lang::Fr) => "Transformation",
        ("transform", Lang::En) => "Transform",
        ("transform", Lang::Zh) => "变换",
        ("position", Lang::Fr) => "Position",
        ("position", Lang::En) => "Position",
        ("position", Lang::Zh) => "位置",
        ("rotation", Lang::Fr) => "Rotation",
        ("rotation", Lang::En) => "Rotation",
        ("rotation", Lang::Zh) => "旋转",
        ("scale", Lang::Fr) => "Échelle",
        ("scale", Lang::En) => "Scale",
        ("scale", Lang::Zh) => "缩放",
        ("components", Lang::Fr) => "Composants",
        ("components", Lang::En) => "Components",
        ("components", Lang::Zh) => "组件",
        ("name", Lang::Fr) => "Nom",
        ("name", Lang::En) => "Name",
        ("name", Lang::Zh) => "名称",
        ("visible", Lang::Fr) => "Visible",
        ("visible", Lang::En) => "Visible",
        ("visible", Lang::Zh) => "可见",
        ("bones", Lang::Fr) => "Os",
        ("bones", Lang::En) => "Bones",
        ("bones", Lang::Zh) => "骨骼",

        // === Camera ===
        ("camera_free", Lang::Fr) => "Libre",
        ("camera_free", Lang::En) => "Free",
        ("camera_free", Lang::Zh) => "自由",
        ("camera_orbit", Lang::Fr) => "Orbite",
        ("camera_orbit", Lang::En) => "Orbit",
        ("camera_orbit", Lang::Zh) => "环绕",
        ("camera_third", Lang::Fr) => "Troisième personne",
        ("camera_third", Lang::En) => "Third Person",
        ("camera_third", Lang::Zh) => "第三人称",

        // === General ===
        ("search", Lang::Fr) => "Rechercher...",
        ("search", Lang::En) => "Search...",
        ("search", Lang::Zh) => "搜索...",
        ("no_selection", Lang::Fr) => "Aucune sélection",
        ("no_selection", Lang::En) => "No selection",
        ("no_selection", Lang::Zh) => "未选择",
        ("skeleton", Lang::Fr) => "Squelette",
        ("skeleton", Lang::En) => "Skeleton",
        ("skeleton", Lang::Zh) => "骨架",
        ("mesh", Lang::Fr) => "Maillage",
        ("mesh", Lang::En) => "Mesh",
        ("mesh", Lang::Zh) => "网格",
        ("grid", Lang::Fr) => "Grille",
        ("grid", Lang::En) => "Grid",
        ("grid", Lang::Zh) => "网格",
        ("velocities", Lang::Fr) => "Vélocités",
        ("velocities", Lang::En) => "Velocities",
        ("velocities", Lang::Zh) => "速度向量",
        ("axes", Lang::Fr) => "Axes",
        ("axes", Lang::En) => "Axes",
        ("axes", Lang::Zh) => "坐标轴",
        ("gizmo", Lang::Fr) => "Gizmo",
        ("gizmo", Lang::En) => "Gizmo",
        ("gizmo", Lang::Zh) => "控制器",
        ("select_tool", Lang::Fr) => "Sélection",
        ("select_tool", Lang::En) => "Select",
        ("select_tool", Lang::Zh) => "选择",
        ("move_tool", Lang::Fr) => "Déplacer",
        ("move_tool", Lang::En) => "Move",
        ("move_tool", Lang::Zh) => "移动",
        ("rotate_tool", Lang::Fr) => "Rotation",
        ("rotate_tool", Lang::En) => "Rotate",
        ("rotate_tool", Lang::Zh) => "旋转工具",
        ("measure_tool", Lang::Fr) => "Mesurer",
        ("measure_tool", Lang::En) => "Measure",
        ("measure_tool", Lang::Zh) => "测量",
        ("walk_speed", Lang::Fr) => "Vitesse marche",
        ("walk_speed", Lang::En) => "Walk Speed",
        ("walk_speed", Lang::Zh) => "步行速度",
        ("quit", Lang::Fr) => "Quitter",
        ("quit", Lang::En) => "Quit",
        ("quit", Lang::Zh) => "退出",
        ("shortcuts", Lang::Fr) => "Raccourcis",
        ("shortcuts", Lang::En) => "Shortcuts",
        ("shortcuts", Lang::Zh) => "快捷键",
        ("about", Lang::Fr) => "À propos",
        ("about", Lang::En) => "About",
        ("about", Lang::Zh) => "关于",
        ("undo", Lang::Fr) => "Annuler",
        ("undo", Lang::En) => "Undo",
        ("undo", Lang::Zh) => "撤销",
        ("redo", Lang::Fr) => "Refaire",
        ("redo", Lang::En) => "Redo",
        ("redo", Lang::Zh) => "重做",
        ("delete", Lang::Fr) => "Supprimer",
        ("delete", Lang::En) => "Delete",
        ("delete", Lang::Zh) => "删除",
        ("snap_grid", Lang::Fr) => "Aimanter à la grille",
        ("snap_grid", Lang::En) => "Snap to Grid",
        ("snap_grid", Lang::Zh) => "吸附到网格",
        ("drop_hint", Lang::Fr) => "Glissez un fichier ici",
        ("drop_hint", Lang::En) => "Drop a file here",
        ("drop_hint", Lang::Zh) => "拖放文件到此处",

        // === Render Settings ===
        ("render_settings", Lang::Fr) => "Paramètres de rendu",
        ("render_settings", Lang::En) => "Render Settings",
        ("render_settings", Lang::Zh) => "渲染设置",
        ("lighting", Lang::Fr) => "Éclairage",
        ("lighting", Lang::En) => "Lighting",
        ("lighting", Lang::Zh) => "光照",
        ("exposure", Lang::Fr) => "Exposition",
        ("exposure", Lang::En) => "Exposure",
        ("exposure", Lang::Zh) => "曝光",
        ("sun_strength", Lang::Fr) => "Intensité soleil",
        ("sun_strength", Lang::En) => "Sun Strength",
        ("sun_strength", Lang::Zh) => "太阳强度",
        ("sky_strength", Lang::Fr) => "Intensité ciel",
        ("sky_strength", Lang::En) => "Sky Strength",
        ("sky_strength", Lang::Zh) => "天空强度",
        ("ground_strength", Lang::Fr) => "Intensité sol",
        ("ground_strength", Lang::En) => "Ground Strength",
        ("ground_strength", Lang::Zh) => "地面强度",
        ("ambient_strength", Lang::Fr) => "Ambiance",
        ("ambient_strength", Lang::En) => "Ambient",
        ("ambient_strength", Lang::Zh) => "环境光",
        ("light_direction", Lang::Fr) => "Direction lumière",
        ("light_direction", Lang::En) => "Light Direction",
        ("light_direction", Lang::Zh) => "光照方向",
        ("sun_color", Lang::Fr) => "Couleur soleil",
        ("sun_color", Lang::En) => "Sun Color",
        ("sun_color", Lang::Zh) => "太阳颜色",
        ("shadows", Lang::Fr) => "Ombres",
        ("shadows", Lang::En) => "Shadows",
        ("shadows", Lang::Zh) => "阴影",
        ("enabled", Lang::Fr) => "Activé",
        ("enabled", Lang::En) => "Enabled",
        ("enabled", Lang::Zh) => "启用",
        ("shadow_bias", Lang::Fr) => "Biais ombre",
        ("shadow_bias", Lang::En) => "Shadow Bias",
        ("shadow_bias", Lang::Zh) => "阴影偏移",
        ("ssao_radius", Lang::Fr) => "Rayon SSAO",
        ("ssao_radius", Lang::En) => "SSAO Radius",
        ("ssao_radius", Lang::Zh) => "SSAO半径",
        ("ssao_intensity", Lang::Fr) => "Intensité SSAO",
        ("ssao_intensity", Lang::En) => "SSAO Intensity",
        ("ssao_intensity", Lang::Zh) => "SSAO强度",
        ("ssao_bias", Lang::Fr) => "Biais SSAO",
        ("ssao_bias", Lang::En) => "SSAO Bias",
        ("ssao_bias", Lang::Zh) => "SSAO偏移",
        ("bloom_intensity", Lang::Fr) => "Intensité bloom",
        ("bloom_intensity", Lang::En) => "Bloom Intensity",
        ("bloom_intensity", Lang::Zh) => "泛光强度",
        ("bloom_spread", Lang::Fr) => "Étendue bloom",
        ("bloom_spread", Lang::En) => "Bloom Spread",
        ("bloom_spread", Lang::Zh) => "泛光扩散",
        ("reset_defaults", Lang::Fr) => "⟲ Réinitialiser",
        ("reset_defaults", Lang::En) => "⟲ Reset Defaults",
        ("reset_defaults", Lang::Zh) => "⟲ 重置默认",

        // Fallback
        (_key, _) => {
            // Return key itself for unknown translations
            // This is safe because the key is &str with 'static not guaranteed...
            // We'll return a placeholder
            "???"
        }
    }
}
