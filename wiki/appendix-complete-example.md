# Appendix A: Complete Example

For other examples, see provided [scripts](../tests/scripts/).

```rust
mod example.notes

import io.{read_file, write_file};
import ui.{event};

effect alias app = <io, ui, exn<AppError>>

type AppError =
  | IoFailure(io.IoError)
  | ParseFailure(String)
  | NotFound(Int)

@derive(Eq, Show, Clone)
type Note = {
  id: Int,
  title: String,
  content: String,
  updated_at: Int,
}

const NOTES_FILE: String = "notes.json";

fn load_all() -> <app> List<Note> {
  let raw = read_file(NOTES_FILE);
  let raw = handle_io_err(raw)?;
  parse_notes(raw)?
}

fn save_all(notes: &List<Note>) -> <app> Unit {
  let serialized = serialize(notes);
  let r = write_file(NOTES_FILE, serialized);
  handle_io_err(r)?;
  event.emit("notes-changed", notes)?
}

fn find_by_id(id: Int) -> <app> Note {
  let notes = load_all()?;
  region r {
    match notes.iter().find(|n| n.id == id) {
      Some(n) => n.clone(),
      None => throw AppError.NotFound(id),
    }
  }
}

fn handle_io_err<T>(r: Result<T, io.IoError>) -> <exn<AppError>> T {
  match r {
    Ok(v) => v,
    Err(e) => throw AppError.IoFailure(e),
  }
}

@export
fn create_note(title: String, content: String) -> <app> Note {
  let mut notes = load_all()?;
  let new_note = Note {
    id: notes.len() + 1,
    title,
    content,
    updated_at: now(),
  };
  notes = notes.push(new_note.clone());
  save_all(&notes)?;
  new_note
}

@export
fn list_notes() -> <app> List<Note> {
  load_all()
}

@export
fn delete_note(id: Int) -> <app> Unit {
  let notes = load_all()?;
  let filtered = notes.filter(|n| n.id != id);
  save_all(&filtered)
}
```
