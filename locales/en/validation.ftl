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
