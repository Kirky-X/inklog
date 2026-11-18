# Path validation messages

validation-path_traversal = Path traversal detected
validation-dangerous_component = Dangerous path component: { $component }
validation-no_absolute = Absolute paths are not allowed
validation-no_symlinks = Symlinks are not allowed
validation-outside_base = Path is outside base directory

# Built-in sanitizer regex compile invariants (panic surface)
sanitize-ansi_sgr_invariant = hardcoded ANSI SGR regex is valid
sanitize-card_number_invariant = hardcoded card number regex is valid
sanitize-email_invariant = hardcoded email regex is valid
sanitize-ssn_invariant = hardcoded SSN regex is valid
sanitize-password_invariant = hardcoded password regex is valid
sanitize-token_invariant = hardcoded token regex is valid
sanitize-api_key_invariant = hardcoded api_key regex is valid
sanitize-bearer_token_invariant = hardcoded Bearer token regex is valid
sanitize-basic_auth_invariant = hardcoded Basic auth regex is valid

# LoggerBuilder validation errors (accumulated into validation_errors)

builder-invalid-level = Invalid log level '{ $level }'. Valid levels: { $levels }
builder-driver-conflict-explicit = Database driver '{ $driver }' (explicitly set) conflicts with driver '{ $inferred }' inferred from URL '{ $url }'. The inferred driver takes precedence.
builder-driver-conflict = Database driver '{ $driver }' conflicts with driver '{ $inferred }' inferred from URL '{ $url }'.
builder-http-port-invalid = HTTP port must be between 1 and 65535
builder-http-error-mode-unknown = Unknown HTTP error mode '{ $mode }'. Valid modes: warn, strict. Using default 'strict'.
