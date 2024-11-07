set -o errexit
set -o nounset
set -o pipefail

# Check if the package was installed
if nix-env -q razer-laptop-control; then
  echo "razer-laptop-control package installed"
else
  echo "razer-laptop-control package not installed"
  exit 1
fi

# Aparently the service is not enabled by default
# printf "Checking that the service is enabled: "
# systemctl --user is-enabled razercontrol.service

echo "Checking files on the path"
printf -- "- " && which razer-cli
printf -- "- " && which razer-settings

echo "Done!"
