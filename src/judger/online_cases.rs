// Run ad-hoc online testcases without checker scoring.
use crate::error::Result;
use crate::fs;
use crate::fs::get_text_by_path;
use crate::judger::judge_result::{JudgeResult, JudgeResultItem};
use crate::judger::runtime::{
    ResourceLimits, RunPaths, build_run_config, map_runtime_status, run_with_interactor,
};
use crate::schema::OnlineCase;

pub(crate) async fn run_online_cases(
    testcases: Vec<OnlineCase>,
    judge_config: &judger::Config,
    exec_path: &str,
    args: &[String],
    tmp_dir: &std::path::Path,
) -> Result<JudgeResult> {
    let mut out_vec = vec![];
    for case in testcases {
        let input_path = tmp_dir.join(format!("{}.in", case.id));
        fs::write_to_file(&input_path, &case.r#in).await?;

        let config = build_run_config(
            judge_config,
            RunPaths {
                exec_path,
                args,
                input_path: &input_path,
                case_id: case.id,
                tmp_dir,
            },
            ResourceLimits {
                time_limit_ms: None,
                memory_limit_kb: None,
            },
        );

        let res = run_with_interactor(&config, None)?;
        let out_content = get_text_by_path(&config.output_path, None).await?;
        let (sys, r#type) = map_runtime_status(res.result);

        out_vec.push(JudgeResultItem {
            id: case.id,
            time: res.cpu_time,
            mem: res.memory,
            sys,
            r#in: case.r#in,
            ans: String::new(),
            out: out_content,
            score: 0,
            r#type,
        });
    }

    Ok(JudgeResult::MaybeError {
        results: out_vec,
        subtask_configs: Vec::new(),
        total_score: None,
    })
}
