set -o errexit
set -o nounset
set -o pipefail

# Aparently the service is not enabled by default
# printf "Checking that the service is enabled: "
# systemctl --user is-enabled razercontrol.service

echo "Checking files on the path"
printf -- "- " && which razer-cli
printf -- "- " && which razer-settings

echo "Done!"
