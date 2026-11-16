<!-- SPDX-License-Identifier: MIT -->

# hexagonrpc

Upstream [linux-msm/hexagonrpc](https://github.com/linux-msm/hexagonrpc) packaged for `aarch64`. Provides `libhexagonrpc`, `hexagonrpcd` (`hexagonrpcd-sdsp.service`, `hexagonrpcd-adsp-sensorspd.service`, `hexagonrpcd-adsp-rootpd.service`), the `fastrpc` system user (`sysusers.d`), and udev permissions (`0660`, group `fastrpc`) on `/dev/fastrpc-*`.

Device-specific served files (sensor JSON configs, persist registry, and `socinfo` links) belong on the phone under `/usr/share/qcom/<soc>/<vendor>/<device>/` rather than in git, paired with a device systemd drop-in passing `-R <root>`.
