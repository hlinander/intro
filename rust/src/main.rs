use pipewire::{Context, MainLoop};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // println!("Hello, world!");
    let mainloop = MainLoop::new()?;
    let context = Context::new(&mainloop)?;
    let core = context.connect(None)?;
    let registry = core.get_registry()?;

    Ok(())
}
