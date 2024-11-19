use crate::source::Animation;
use anyhow::{Context, Result};
use indoc::indoc;
use std::path::Path;
use tera::{Context as TeraContext, Tera};

pub fn make_header(src_file_stem: &str, src_anims: &[Animation]) -> Result<String> {
    let guard_iden = make_header_guard_iden(src_file_stem);
    let namespace_iden = make_header_namespace_iden(src_file_stem);
    let enum_members = make_header_enum_members(src_anims)?;
    render_header(&guard_iden, &namespace_iden, &enum_members)
}

fn make_header_guard_iden(src_file_stem: &str) -> String {
    format!("{}_ANIM_H__", src_file_stem.to_uppercase())
}

fn make_header_namespace_iden(src_file_stem: &str) -> String {
    format!("{}_Anim", src_file_stem)
}

fn make_header_enum_members(src_anims: &[Animation]) -> Result<Vec<String>> {
    let mut result = Vec::new();
    for anim in src_anims.iter() {
        let adj_anim_path = anim.path.replace("\\", "/").replace("-", "_");
        let enum_name = Path::new(&adj_anim_path)
            .file_stem()
            .context("file stem")?
            .to_string_lossy()
            .to_uppercase();
        let enum_value = anim.id;
        let enum_member = format!("{} = {}", enum_name, enum_value);
        result.push(enum_member);
    }
    Ok(result)
}

fn render_header(
    guard_iden: &str,
    namespace_iden: &str,
    enum_members: &[String],
) -> Result<String> {
    let header_template = indoc! {"
        // This file was automatically generated. It contains definitions for all the
        // animations stored in the associated KFM file. Include this file in your
        // final application to easily refer to animation sequences.

        #ifndef {{ guard_iden }}
        #define {{ guard_iden }}

        namespace {{ namespace_iden }}
        {
            enum
            {
            {%- for member in enum_members %}
                {{ member }}{% if not loop.last %},{% endif %}
            {%- endfor %}
            };
        }

        #endif  // #ifndef {{ guard_iden }}
    "};

    let mut tera = Tera::default();
    tera.add_raw_template("header", header_template)?;

    let mut tera_ctx = TeraContext::new();
    tera_ctx.insert("guard_iden", &guard_iden);
    tera_ctx.insert("namespace_iden", &namespace_iden);
    tera_ctx.insert("enum_members", &enum_members);

    let result = tera.render("header", &tera_ctx)?;
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::make_header;
    use crate::source::Animation;
    use crate::source::{Transition, TransitionType};
    use indoc::indoc;

    #[test]
    fn test_make_header_file() {
        let src_path = "inventor_broombot";
        let src_anim_paths = [
            "./mech/mech_gunbot_m_idle.kf",
            "./mech/mech_gunbot_m_run.kf",
            "./mech/mech_gunbot_a_attack.kf",
            "./mech/mech_gunbot_h_onhit.kf",
        ];
        let mut src_anims = Vec::new();
        for (id, path) in src_anim_paths.iter().enumerate() {
            // Create animation from `id` and `path`
            let mut anim = Animation {
                id: id as u32,
                path: path.to_string(),
                index: 0,
                trans: Vec::new(),
            };
            // Insert transition to every other animation
            for (trans_id, _) in src_anim_paths.iter().enumerate() {
                if trans_id != id {
                    anim.trans.push(Transition {
                        id: trans_id as u32,
                        type_: TransitionType::DefaultNonSync,
                        ext: None,
                    });
                }
            }
            src_anims.push(anim);
        }

        let expected = indoc! {"
            // This file was automatically generated. It contains definitions for all the
            // animations stored in the associated KFM file. Include this file in your
            // final application to easily refer to animation sequences.

            #ifndef INVENTOR_BROOMBOT_ANIM_H__
            #define INVENTOR_BROOMBOT_ANIM_H__

            namespace inventor_broombot_Anim
            {
                enum
                {
                    MECH_GUNBOT_M_IDLE = 0,
                    MECH_GUNBOT_M_RUN = 1,
                    MECH_GUNBOT_A_ATTACK = 2,
                    MECH_GUNBOT_H_ONHIT = 3
                };
            }

            #endif  // #ifndef INVENTOR_BROOMBOT_ANIM_H__
        "};
        let actual = make_header(src_path, &src_anims).unwrap();
        assert_eq!(expected, actual);
    }
}
