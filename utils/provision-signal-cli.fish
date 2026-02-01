#!/usr/bin/env fish
#
# provision-signal-cli.fish - Standalone Signal Bot Number Provisioning Utility
#
# ⚠️  IMPORTANT: This script is NOT part of Stroma.
#     It is a standalone convenience tool for operators who want to obtain
#     a DEDICATED phone number for their bot via SMSpool.
#
# ℹ️  RECOMMENDATION: Most operators should use "Link as Secondary Device" instead.
#     See docs/OPERATOR-GUIDE.md § "Signal Registration" for details.
#     - Link Device: Simpler, no dedicated phone needed, full capabilities
#     - Primary Device: Use this script only if you need a separate identity
#
# Purpose:
#   - Provision a temporary phone number via SMSpool (Phase 1)
#   - Register with Signal using provided CAPTCHA token (Phase 2)
#   - Automatically retrieve SMS code and complete verification
#
# When to Use This Script:
#   - You want the bot to have its OWN phone number (not your personal)
#   - You don't have access to a prepaid SIM card
#   - Testing with disposable numbers
#
# When NOT to Use This Script:
#   - Production deployments (use prepaid SIM instead)
#   - Most operators (just link to existing Signal account)
#   - Privacy-sensitive deployments (SMSpool is a third party)
#
# Requirements:
#   - xh (modern HTTP client, preferred) or curl
#   - jq (JSON parsing)
#   - signal-cli (Signal protocol client)
#   - SMSpool API key (https://www.smspool.net/)
#
# Usage (Two-Phase Workflow):
#   # Phase 1: Provision a phone number
#   ./provision-signal-cli.fish --provision-number
#
#   # Phase 2: Register with Signal (CAPTCHA as CLI argument)
#   ./provision-signal-cli.fish --phone +12025551234 --order-id ABCD1234 \
#       --captcha 'signalcaptcha://...'
#
# Why Two Phases?
#   macOS has a 1024-byte TTY input buffer. CAPTCHA tokens are ~2000 bytes.
#   Passing via CLI argument bypasses this kernel limitation (shell handles expansion).
#
# Security Notes:
#   - Never commit SMSPOOL_API_KEY to git
#   - Store credentials securely after generation
#   - Consider this number disposable (SMSpool numbers can be recycled)
#   - For production: use a dedicated prepaid SIM instead
#

# Configuration
set -g SMSPOOL_BASE_URL "https://api.smspool.net"
set -g SIGNAL_CAPTCHA_URL "https://signalcaptchas.org/registration/generate.html"
set -g SIGNAL_SERVICE_ID "829"  # Signal service ID in SMSpool

# State file for convenience
set -g STATE_FILE "/tmp/signal_provision_state.env"

# Colors
set -g RED '\033[0;31m'
set -g GREEN '\033[0;32m'
set -g YELLOW '\033[1;33m'
set -g CYAN '\033[0;36m'
set -g NC '\033[0m'

function print_banner
    echo ""
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    echo "  Signal Bot Number Provisioning Utility"
    echo "  ⚠️  STANDALONE TOOL - NOT PART OF STROMA"
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    echo ""
end

function print_usage
    echo "Usage:"
    echo ""
    echo "  Phase 1: Provision a phone number"
    echo "    provision-signal-cli.fish --provision-number [--country CODE]"
    echo ""
    echo "  Phase 2: Register with Signal"
    echo "    provision-signal-cli.fish \\"
    echo "      --phone +12025551234 \\"
    echo "      --order-id ABCD1234 \\"
    echo "      --captcha 'signalcaptcha://...'"
    echo ""
    echo "Options:"
    echo "  --provision-number       Provision a phone number from SMSpool"
    echo "  --country CODE           Country code for number (default: US)"
    echo "  --phone NUMBER           Phone number in E.164 format"
    echo "  --order-id ID            SMSpool order ID (from Phase 1)"
    echo "  --captcha TOKEN          Signal CAPTCHA token URL"
    echo "  --skip-username          Skip username setup"
    echo "  --help                   Show this help"
    echo ""
    echo "Environment Variables:"
    echo "  SMSPOOL_API_KEY          Required: Your SMSpool API key"
    echo "  SIGNAL_CAPTCHA           Alternative to --captcha flag"
    echo ""
    echo "Examples:"
    echo "  # Get a phone number"
    echo "  set -gx SMSPOOL_API_KEY \"your_key\""
    echo "  ./provision-signal-cli.fish --provision-number"
    echo ""
    echo "  # Register with Signal"
    echo "  ./provision-signal-cli.fish \\"
    echo "    --phone +12025551234 \\"
    echo "    --order-id ABCD1234 \\"
    echo "    --captcha 'signalcaptcha://signal-hcaptcha...'"
    echo ""
end

function check_dependencies
    # Check for required commands
    if not command -v jq >/dev/null
        echo -e "$RED✗ jq not found. Install with: brew install jq$NC"
        return 1
    end
    
    if not command -v signal-cli >/dev/null
        echo -e "$RED✗ signal-cli not found. Install with: brew install signal-cli$NC"
        return 1
    end
    
    # Check for HTTP client (prefer xh)
    if command -v xh >/dev/null
        set -g HTTP_CLIENT xh
    else if command -v curl >/dev/null
        set -g HTTP_CLIENT curl
    else
        echo -e "$RED✗ No HTTP client found. Install xh: brew install xh$NC"
        return 1
    end
    
    return 0
end

function http_post
    set -l url $argv[1]
    set -l form_data $argv[2..-1]
    
    if test "$HTTP_CLIENT" = "xh"
        # Use --form flag to send as application/x-www-form-urlencoded
        xh --form POST $url $form_data
    else
        # Build curl command with form data
        set -l curl_args
        for item in $form_data
            set curl_args $curl_args -d $item
        end
        curl -X POST $url $curl_args
    end
end

function provision_number
    set -l country $argv[1]
    
    echo -e "$CYAN━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━$NC"
    echo -e "$CYAN Phase 1: Provisioning Phone Number$NC"
    echo -e "$CYAN━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━$NC"
    echo ""
    echo "   Country: $country"
    echo "   Service: Signal"
    echo ""
    
    # Check for API key
    if not set -q SMSPOOL_API_KEY; or test -z "$SMSPOOL_API_KEY"
        echo -e "$RED✗ SMSPOOL_API_KEY not set$NC"
        echo "  Set it with: set -gx SMSPOOL_API_KEY \"your_key\""
        return 1
    end
    
    # Call SMSpool API
    echo "Calling SMSpool API..."
    echo "  Endpoint: $SMSPOOL_BASE_URL/purchase/sms"
    echo "  Country: $country"
    echo "  Service: $SIGNAL_SERVICE_ID"
    echo ""
    
    set -l response (http_post "$SMSPOOL_BASE_URL/purchase/sms" \
        "key=$SMSPOOL_API_KEY" \
        "country=$country" \
        "service=$SIGNAL_SERVICE_ID")
    
    # Debug output
    echo "API Response:"
    echo $response | jq '.'
    echo ""
    
    # Check if successful
    set -l success (echo $response | jq -r '.success')
    if test "$success" != "1"
        set -l message (echo $response | jq -r '.message // "Unknown error"')
        echo -e "$RED✗ Failed to provision number: $message$NC"
        echo ""
        echo "Debug info:"
        echo "  - Verify SMSPOOL_API_KEY is correct"
        echo "  - Check API documentation: https://www.smspool.net/article/how-to-use-the-smspool-api"
        echo "  - Service ID $SIGNAL_SERVICE_ID may be incorrect"
        echo ""
        return 1
    end
    
    # Extract number and order ID
    set -l number (echo $response | jq -r '.number')
    set -l order_id (echo $response | jq -r '.order_id')
    
    # Add + prefix if not present
    if not string match -q '+*' $number
        set number "+$number"
    end
    
    echo ""
    echo -e "$GREEN✅ Number provisioned: $number$NC"
    echo "   Order ID: $order_id"
    echo ""
    
    # Save state for convenience
    echo "# Signal provisioning state - source this file" > $STATE_FILE
    echo "set -gx PHONE \"$number\"" >> $STATE_FILE
    echo "set -gx ORDER_ID \"$order_id\"" >> $STATE_FILE
    
    echo -e "$CYAN━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━$NC"
    echo -e "$CYAN NEXT STEPS:$NC"
    echo -e "$CYAN━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━$NC"
    echo ""
    echo "  1. Get a CAPTCHA token from:"
    echo "     $SIGNAL_CAPTCHA_URL"
    echo ""
    echo "  TIP: Complete the captcha, and right-click \\"Open Signal\\" and save to clipboard."
    echo "       Then paste the token into the --captcha flag."
    echo ""
    echo "  2. Complete registration with:"
    echo ""
    echo "     \$ ./provision-signal-cli.fish \\"
    echo "       --phone $number \\"
    echo "       --order-id $order_id \\"
    echo "       --captcha 'signalcaptcha://...'"
    echo ""
    echo "  Or use environment variable:"
    echo ""
    echo "     \$ set -gx SIGNAL_CAPTCHA 'signalcaptcha://...'"
    echo "     \$ ./provision-signal-cli.fish \\"
    echo "       --phone $number \\"
    echo "       --order-id $order_id"
    echo ""
    echo -e "$CYAN━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━$NC"
    echo ""
    echo -e "$YELLOW💡 TIP: Phone number and order ID saved to:$NC"
    echo "    $STATE_FILE"
    echo ""
    echo "    You can source this file:"
    echo "    \$ source $STATE_FILE"
    echo "    \$ ./provision-signal-cli.fish --phone \$PHONE --order-id \$ORDER_ID --captcha '...'"
    echo ""
    
    return 0
end

function register_with_signal
    set -l phone $argv[1]
    set -l order_id $argv[2]
    set -l captcha $argv[3]
    
    echo -e "$CYAN━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━$NC"
    echo -e "$CYAN Phase 2: Registering with Signal$NC"
    echo -e "$CYAN━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━$NC"
    echo ""
    echo "   Number: $phone"
    echo ""
    
    # Initiate registration with Signal
    echo "Calling Signal registration API..."
    echo "(This may take 10-30 seconds)"
    echo ""
    
    signal-cli -a $phone register --captcha $captcha
    
    if test $status -ne 0
        echo -e "$RED✗ Failed to register with Signal$NC"
        echo ""
        echo "Possible causes:"
        echo "  1. CAPTCHA token expired (get fresh one from signalcaptchas.org)"
        echo "  2. Number already registered"
        echo "  3. Rate limiting from Signal (wait and try again)"
        echo ""
        return 1
    end
    
    echo ""
    echo -e "$GREEN✅ Registration initiated!$NC"
    echo "   Signal will send SMS verification code"
    echo ""
    
    # Wait for SMS code
    echo -e "$CYAN━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━$NC"
    echo -e "$CYAN Retrieving SMS Verification Code$NC"
    echo -e "$CYAN━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━$NC"
    echo ""
    echo "   Waiting for SMS from Signal (checking SMSpool)..."
    echo ""
    
    set -l max_attempts 30  # 30 * 5s = 150s = 2.5 minutes
    set -l code ""
    
    printf "   Checking"
    for attempt in (seq 1 $max_attempts)
        sleep 5
        
        # Check SMSpool for SMS
        set -l response (http_post "$SMSPOOL_BASE_URL/request/active" \
            "key=$SMSPOOL_API_KEY" \
            "orderid=$order_id")
        
        # Find completed message with code
        set code (echo $response | jq -r '.[] | select(.order_code == "'$order_id'" and .status == "completed" and .code != "0") | .code' | head -1)
        
        if test -n "$code"
            break
        end
        
        printf "."
    end
    
    echo ""
    echo ""
    
    if test -z "$code"
        echo -e "$RED✗ Timeout waiting for SMS$NC"
        echo "  Check SMSpool dashboard manually: https://www.smspool.net/"
        return 1
    end
    
    echo -e "$GREEN✅ SMS received!$NC"
    echo "   Code: $code"
    echo ""
    
    # Verify registration
    echo -e "$CYAN━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━$NC"
    echo -e "$CYAN Verifying Registration$NC"
    echo -e "$CYAN━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━$NC"
    echo ""
    
    signal-cli -a $phone verify $code
    
    if test $status -ne 0
        echo -e "$RED✗ Failed to verify registration$NC"
        return 1
    end
    
    echo ""
    echo -e "$GREEN✅ Signal registration complete!$NC"
    echo ""
    
    return 0
end

function setup_username
    set -l phone $argv[1]
    
    echo -e "$CYAN━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━$NC"
    echo -e "$CYAN Optional Username Setup$NC"
    echo -e "$CYAN━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━$NC"
    echo ""
    echo "Signal usernames allow people to contact you without"
    echo "knowing your phone number."
    echo ""
    
    read -l -P "Would you like to set a username now? [y/N] " response
    
    if not test "$response" = "y"; and not test "$response" = "Y"
        echo ""
        echo "Skipping username setup."
        echo "You can set it later with:"
        echo "  signal-cli -a $phone updateAccount --username YOUR_USERNAME"
        echo ""
        return 0
    end
    
    read -l -P "Enter desired username (lowercase, numbers, underscores): " username
    
    if test -z "$username"
        echo "Skipping - no username provided"
        return 0
    end
    
    echo ""
    echo "Setting username to: $username"
    
    signal-cli -a $phone updateAccount --username $username
    
    if test $status -ne 0
        echo -e "$YELLOW⚠️  Username setup failed$NC"
        echo "You can try again later with:"
        echo "  signal-cli -a $phone updateAccount --username $username"
    else
        echo -e "$GREEN✅ Username set!$NC"
    end
    
    echo ""
    
    return 0
end

function output_credentials
    set -l phone $argv[1]
    
    echo ""
    echo -e "$GREEN━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━$NC"
    echo -e "$GREEN✅ Signal Account Created$NC"
    echo -e "$GREEN━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━$NC"
    echo ""
    echo -e "📱 Phone Number: $GREEN$phone$NC"
    echo ""
    echo -e "$YELLOW━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━$NC"
    echo -e "$YELLOW⚠️  NEXT STEP: Link Stroma Bot to This Account$NC"
    echo -e "$YELLOW━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━$NC"
    echo ""
    echo "1. This signal-cli session is your PRIMARY device."
    echo "   Credentials stored in: ~/.local/share/signal-cli/data/$phone/"
    echo ""
    echo "2. Link Stroma as secondary device:"
    echo ""
    echo "   # Start Stroma linking process"
    echo "   stroma link-device --device-name \"Stroma Bot\""
    echo ""
    echo "   # A QR code will appear. To scan it with signal-cli:"
    echo "   signal-cli -a $phone addDevice --uri \"sgnl://linkdevice?...\""
    echo ""
    echo "   # Or use Signal mobile app (if you have one linked):"
    echo "   # Signal → Settings → Linked Devices → Link New Device"
    echo ""
    echo "3. After linking, follow docs/OPERATOR-GUIDE.md for bootstrap"
    echo ""
    echo "3. SMSpool Number Notes:"
    echo "   ⚠️  Temporary/disposable number (may be recycled)"
    echo "   ⚠️  For production: use dedicated phone number"
    echo "   Cost: ~\$0.50-\$2.00 per number"
    echo ""
    echo -e "$CYAN━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━$NC"
    echo "Documentation"
    echo -e "$CYAN━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━$NC"
    echo ""
    echo "  • Stroma Deployment: docs/OPERATOR-GUIDE.md"
    echo "  • Manual Registration: docs/SIGNAL_BOT_REGISTRATION.md"
    echo ""
end

# Main script
print_banner

# Parse arguments
set -l mode ""
set -l country "US"
set -l phone ""
set -l order_id ""
set -l captcha ""
set -l skip_username 0

for i in (seq (count $argv))
    switch $argv[$i]
        case --provision-number
            set mode "provision"
        case --country
            set country $argv[(math $i + 1)]
        case --phone
            set phone $argv[(math $i + 1)]
        case --order-id
            set order_id $argv[(math $i + 1)]
        case --captcha
            set captcha $argv[(math $i + 1)]
        case --skip-username
            set skip_username 1
        case --help -h
            print_usage
            exit 0
    end
end

# Check for CAPTCHA from environment if not provided
if test -z "$captcha"; and set -q SIGNAL_CAPTCHA
    set captcha $SIGNAL_CAPTCHA
end

# Check dependencies
if not check_dependencies
    exit 1
end

# Execute based on mode
if test "$mode" = "provision"
    # Phase 1: Provision number
    if not provision_number $country
        exit 1
    end
else if test -n "$phone"; and test -n "$order_id"; and test -n "$captcha"
    # Phase 2: Register with Signal
    if not register_with_signal $phone $order_id $captcha
        exit 1
    end
    
    # Optional username setup
    if test $skip_username -eq 0
        setup_username $phone
    end
    
    # Output credentials
    output_credentials $phone
else
    echo -e "$RED✗ Invalid arguments$NC"
    echo ""
    print_usage
    exit 1
end
