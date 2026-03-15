Output demo.gif

Require neo

Set Shell "bash"
Set FontSize 14
Set Width 1200
Set Height 700
Set Theme "Catppuccin Mocha"
Set TypingSpeed 40ms
Set Padding 20

Env PATH "/Users/matt/.local/bin:/opt/homebrew/bin:/usr/local/bin:/usr/bin:/bin"
Env OPENROUTER_API_KEY "$OPENROUTER_API_KEY"
Env HOME "/Users/matt"

Hide
Type "cd /Users/matt/neo && clear"
Enter
Sleep 1s
Show

# Show help
Type "neo --help"
Enter
Sleep 3s

# One-shot ask — simple question
Type `neo ask "What is the capital of France? Answer in one sentence."`
Enter
Sleep 10s

# Ask with tool use — reads project files
Type `neo ask "Read src/lib.rs and list every module"`
Enter
Sleep 15s

# Review uncommitted changes
Type "neo review"
Enter
Sleep 15s

# Show cost
Type "neo cost"
Enter
Sleep 3s

Sleep 2s
