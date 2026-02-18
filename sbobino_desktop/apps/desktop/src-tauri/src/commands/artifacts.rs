use std::fs::File;
use std::io::BufWriter;
use std::path::Path;

use docx_rs::{Docx, Paragraph, Run};
use printpdf::{ops::PdfPage, text::TextItem, units::Pt, BuiltinFont, Mm, Op, PdfDocument};
use serde::{Deserialize, Serialize};
use tauri::State;

use sbobino_application::ArtifactQuery;
use sbobino_domain::{ArtifactKind, TranscriptArtifact};

use crate::{error::CommandError, state::AppState};

#[derive(Debug, Deserialize)]
pub struct GetArtifactPayload {
    pub id: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateArtifactPayload {
    pub id: String,
    pub optimized_transcript: String,
    pub summary: String,
    pub faqs: String,
}

#[derive(Debug, Deserialize)]
pub struct ChatArtifactPayload {
    pub id: String,
    pub prompt: String,
}

#[derive(Debug, Deserialize)]
pub struct ListArtifactsPayload {
    pub kind: Option<ArtifactKind>,
    pub query: Option<String>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

#[derive(Debug, Deserialize)]
pub struct RenameArtifactPayload {
    pub id: String,
    pub new_title: String,
}

#[derive(Debug, Deserialize)]
pub struct DeleteArtifactsPayload {
    pub ids: Vec<String>,
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExportFormat {
    Txt,
    Docx,
    Pdf,
}

#[derive(Debug, Deserialize)]
pub struct ExportArtifactPayload {
    pub id: String,
    pub format: ExportFormat,
    pub destination_path: String,
    pub content_override: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ReadAudioFilePayload {
    pub path: String,
}

#[derive(Debug, Serialize)]
pub struct DeleteArtifactsResponse {
    pub deleted: usize,
}

#[derive(Debug, Serialize)]
pub struct RestoreArtifactsResponse {
    pub restored: usize,
}

#[derive(Debug, Serialize)]
pub struct ExportArtifactResponse {
    pub path: String,
}

#[tauri::command]
pub async fn list_recent_artifacts(
    state: State<'_, AppState>,
    limit: Option<usize>,
) -> Result<Vec<TranscriptArtifact>, CommandError> {
    state
        .artifact_service
        .list(ArtifactQuery {
            kind: None,
            query: None,
            limit,
            offset: Some(0),
        })
        .await
        .map_err(CommandError::from)
}

#[tauri::command]
pub async fn list_artifacts(
    state: State<'_, AppState>,
    payload: Option<ListArtifactsPayload>,
) -> Result<Vec<TranscriptArtifact>, CommandError> {
    let payload = payload.unwrap_or(ListArtifactsPayload {
        kind: None,
        query: None,
        limit: Some(100),
        offset: Some(0),
    });

    state
        .artifact_service
        .list(ArtifactQuery {
            kind: payload.kind,
            query: payload.query,
            limit: payload.limit,
            offset: payload.offset,
        })
        .await
        .map_err(CommandError::from)
}

#[tauri::command]
pub async fn list_deleted_artifacts(
    state: State<'_, AppState>,
    payload: Option<ListArtifactsPayload>,
) -> Result<Vec<TranscriptArtifact>, CommandError> {
    let payload = payload.unwrap_or(ListArtifactsPayload {
        kind: None,
        query: None,
        limit: Some(100),
        offset: Some(0),
    });

    state
        .artifact_service
        .purge_deleted_older_than_days(30)
        .await
        .map_err(CommandError::from)?;

    state
        .artifact_service
        .list_deleted(ArtifactQuery {
            kind: payload.kind,
            query: payload.query,
            limit: payload.limit,
            offset: payload.offset,
        })
        .await
        .map_err(CommandError::from)
}

#[tauri::command]
pub async fn get_artifact(
    state: State<'_, AppState>,
    payload: GetArtifactPayload,
) -> Result<Option<TranscriptArtifact>, CommandError> {
    state
        .artifact_service
        .get(&payload.id)
        .await
        .map_err(CommandError::from)
}

#[tauri::command]
pub async fn update_artifact(
    state: State<'_, AppState>,
    payload: UpdateArtifactPayload,
) -> Result<Option<TranscriptArtifact>, CommandError> {
    state
        .artifact_service
        .update_content(
            &payload.id,
            &payload.optimized_transcript,
            &payload.summary,
            &payload.faqs,
        )
        .await
        .map_err(CommandError::from)
}

#[tauri::command]
pub async fn rename_artifact(
    state: State<'_, AppState>,
    payload: RenameArtifactPayload,
) -> Result<Option<TranscriptArtifact>, CommandError> {
    state
        .artifact_service
        .rename(&payload.id, &payload.new_title)
        .await
        .map_err(CommandError::from)
}

#[tauri::command]
pub async fn delete_artifacts(
    state: State<'_, AppState>,
    payload: DeleteArtifactsPayload,
) -> Result<DeleteArtifactsResponse, CommandError> {
    let deleted = state
        .artifact_service
        .delete_many(&payload.ids)
        .await
        .map_err(CommandError::from)?;

    Ok(DeleteArtifactsResponse { deleted })
}

#[tauri::command]
pub async fn restore_artifacts(
    state: State<'_, AppState>,
    payload: DeleteArtifactsPayload,
) -> Result<RestoreArtifactsResponse, CommandError> {
    let restored = state
        .artifact_service
        .restore_many(&payload.ids)
        .await
        .map_err(CommandError::from)?;

    Ok(RestoreArtifactsResponse { restored })
}

#[tauri::command]
pub async fn hard_delete_artifacts(
    state: State<'_, AppState>,
    payload: DeleteArtifactsPayload,
) -> Result<DeleteArtifactsResponse, CommandError> {
    let deleted = state
        .artifact_service
        .hard_delete_many(&payload.ids)
        .await
        .map_err(CommandError::from)?;

    Ok(DeleteArtifactsResponse { deleted })
}

#[tauri::command]
pub async fn empty_deleted_artifacts(
    state: State<'_, AppState>,
) -> Result<DeleteArtifactsResponse, CommandError> {
    let mut offset = 0_usize;
    let mut ids = Vec::new();

    loop {
        let page = state
            .artifact_service
            .list_deleted(ArtifactQuery {
                kind: None,
                query: None,
                limit: Some(500),
                offset: Some(offset),
            })
            .await
            .map_err(CommandError::from)?;

        if page.is_empty() {
            break;
        }

        let page_len = page.len();
        ids.extend(page.into_iter().map(|artifact| artifact.id));

        if page_len < 500 {
            break;
        }
        offset += page_len;
    }

    let deleted = state
        .artifact_service
        .hard_delete_many(&ids)
        .await
        .map_err(CommandError::from)?;

    Ok(DeleteArtifactsResponse { deleted })
}

#[tauri::command]
pub async fn export_artifact(
    state: State<'_, AppState>,
    payload: ExportArtifactPayload,
) -> Result<ExportArtifactResponse, CommandError> {
    let destination_path = Path::new(&payload.destination_path);

    if let Some(content_override) = payload
        .content_override
        .as_ref()
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
    {
        match payload.format {
            ExportFormat::Txt => export_txt(destination_path, content_override)?,
            ExportFormat::Docx => export_docx(destination_path, content_override)?,
            ExportFormat::Pdf => export_pdf(destination_path, content_override)?,
        }

        return Ok(ExportArtifactResponse {
            path: destination_path.to_string_lossy().to_string(),
        });
    }

    let artifact = state
        .artifact_service
        .get(&payload.id)
        .await
        .map_err(CommandError::from)?
        .ok_or_else(|| CommandError::new("not_found", "artifact not found"))?;

    if artifact.optimized_transcript.trim().is_empty()
        && artifact.raw_transcript.trim().is_empty()
    {
        return Err(CommandError::new(
            "empty_content",
            "no transcription available to export",
        ));
    }
    let transcription = if artifact.optimized_transcript.trim().is_empty() {
        artifact.raw_transcript.as_str()
    } else {
        artifact.optimized_transcript.as_str()
    };

    match payload.format {
        ExportFormat::Txt => export_txt(destination_path, transcription)?,
        ExportFormat::Docx => export_docx(destination_path, transcription)?,
        ExportFormat::Pdf => export_pdf(destination_path, transcription)?,
    }

    Ok(ExportArtifactResponse {
        path: destination_path.to_string_lossy().to_string(),
    })
}

#[tauri::command]
pub async fn chat_artifact(
    state: State<'_, AppState>,
    payload: ChatArtifactPayload,
) -> Result<String, CommandError> {
    let artifact = state
        .artifact_service
        .get(&payload.id)
        .await
        .map_err(CommandError::from)?
        .ok_or_else(|| CommandError::new("not_found", "artifact not found"))?;

    let enhancer = state
        .runtime_factory
        .build_gemini_enhancer()
        .map_err(|e| CommandError::new("runtime_factory", e))?
        .ok_or_else(|| {
            CommandError::new(
                "missing_api_key",
                "Gemini API key is not configured in settings.",
            )
        })?;

    let context = format!(
        "You are an assistant for transcript analysis.\n\nTranscript:\n{}\n\nSummary:\n{}\n\nFAQs:\n{}\n\nUser question:\n{}",
        artifact.optimized_transcript,
        artifact.summary,
        artifact.faqs,
        payload.prompt
    );

    enhancer.ask(&context).await.map_err(CommandError::from)
}

#[tauri::command]
pub async fn read_audio_file(payload: ReadAudioFilePayload) -> Result<Vec<u8>, CommandError> {
    tokio::fs::read(&payload.path)
        .await
        .map_err(|e| CommandError::new("audio", format!("failed to read audio file: {e}")))
}

fn export_txt(path: &Path, transcription: &str) -> Result<(), CommandError> {
    std::fs::write(path, transcription)
        .map_err(|e| CommandError::new("export", format!("failed to export txt: {e}")))
}

fn export_docx(path: &Path, transcription: &str) -> Result<(), CommandError> {
    let doc = Docx::new().add_paragraph(Paragraph::new().add_run(Run::new().add_text(transcription)));

    let file = File::create(path)
        .map_err(|e| CommandError::new("export", format!("failed to create docx file: {e}")))?;

    doc.build()
        .pack(file)
        .map_err(|e| CommandError::new("export", format!("failed to write docx: {e}")))
}

fn export_pdf(path: &Path, transcription: &str) -> Result<(), CommandError> {
    let mut doc = PdfDocument::new("Transcription");

    let mut ops = vec![
        Op::StartTextSection,
        Op::SetFontSizeBuiltinFont {
            size: Pt(20.0),
            font: BuiltinFont::HelveticaBold,
        },
        Op::SetTextCursor {
            pos: printpdf::graphics::Point {
                x: Pt(28.0),
                y: Pt(810.0),
            },
        },
        Op::WriteTextBuiltinFont {
            items: vec![TextItem::Text("Transcription".to_string())],
            font: BuiltinFont::HelveticaBold,
        },
    ];

    let mut y = 780.0_f32;
    append_pdf_section(&mut ops, "Transcription", transcription, &mut y);

    ops.push(Op::EndTextSection);

    doc.with_pages(vec![PdfPage::new(Mm(210.0), Mm(297.0), ops)]);

    let mut warnings = Vec::new();
    let bytes = doc.save(
        &printpdf::PdfSaveOptions {
            optimize: true,
            ..Default::default()
        },
        &mut warnings,
    );

    let mut writer = BufWriter::new(
        File::create(path)
            .map_err(|e| CommandError::new("export", format!("failed to create pdf file: {e}")))?,
    );

    std::io::Write::write_all(&mut writer, &bytes)
        .map_err(|e| CommandError::new("export", format!("failed to write pdf: {e}")))
}

fn append_pdf_section(ops: &mut Vec<Op>, title: &str, text: &str, y: &mut f32) {
    if text.trim().is_empty() {
        return;
    }

    ops.push(Op::SetFontSizeBuiltinFont {
        size: Pt(14.0),
        font: BuiltinFont::HelveticaBold,
    });
    ops.push(Op::SetTextCursor {
        pos: printpdf::graphics::Point {
            x: Pt(28.0),
            y: Pt(*y),
        },
    });
    ops.push(Op::WriteTextBuiltinFont {
        items: vec![TextItem::Text(title.to_string())],
        font: BuiltinFont::HelveticaBold,
    });

    *y -= 20.0;

    ops.push(Op::SetFontSizeBuiltinFont {
        size: Pt(11.0),
        font: BuiltinFont::Helvetica,
    });

    for line in text.lines() {
        if *y < 40.0 {
            break;
        }
        ops.push(Op::SetTextCursor {
            pos: printpdf::graphics::Point {
                x: Pt(28.0),
                y: Pt(*y),
            },
        });
        ops.push(Op::WriteTextBuiltinFont {
            items: vec![TextItem::Text(line.to_string())],
            font: BuiltinFont::Helvetica,
        });
        *y -= 14.0;
    }

    *y -= 12.0;
}
