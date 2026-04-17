#!/bin/bash
set -e

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

VERSION_FILE="libloragw/VERSION"
CONFIG_FILE="libloragw/libloragw/inc/config.h"
LIBRARY_CFG="libloragw/libloragw/library.cfg"

if [[ ! -f "$VERSION_FILE" ]]; then
    echo "ERROR: VERSION file not found at $VERSION_FILE" >&2
    exit 1
fi

if [[ ! -f "$LIBRARY_CFG" ]]; then
    echo "ERROR: library.cfg not found at $LIBRARY_CFG" >&2
    exit 1
fi

LIBLORAGW_VERSION="$(cat "$VERSION_FILE")"

DEBUG_AUX=0
DEBUG_COM=0
DEBUG_MCU=0
DEBUG_I2C=0
DEBUG_REG=0
DEBUG_HAL=0
DEBUG_LBT=0
DEBUG_GPS=0
DEBUG_GPIO=""
DEBUG_RAD=0
DEBUG_CAL=0
DEBUG_SX1302=0
DEBUG_FTIME=0

while IFS= read -r line; do
    case "$line" in
        DEBUG_AUX=*|\ DEBUG_AUX=*)
            DEBUG_AUX="${line#*=}"
            DEBUG_AUX="${DEBUG_AUX# }"
            ;;
        DEBUG_COM=*|\ DEBUG_COM=*)
            DEBUG_COM="${line#*=}"
            DEBUG_COM="${DEBUG_COM# }"
            ;;
        DEBUG_MCU=*|\ DEBUG_MCU=*)
            DEBUG_MCU="${line#*=}"
            DEBUG_MCU="${DEBUG_MCU# }"
            ;;
        DEBUG_I2C=*|\ DEBUG_I2C=*)
            DEBUG_I2C="${line#*=}"
            DEBUG_I2C="${DEBUG_I2C# }"
            ;;
        DEBUG_REG=*|\ DEBUG_REG=*)
            DEBUG_REG="${line#*=}"
            DEBUG_REG="${DEBUG_REG# }"
            ;;
        DEBUG_HAL=*|\ DEBUG_HAL=*)
            DEBUG_HAL="${line#*=}"
            DEBUG_HAL="${DEBUG_HAL# }"
            ;;
        DEBUG_LBT=*|\ DEBUG_LBT=*)
            DEBUG_LBT="${line#*=}"
            DEBUG_LBT="${DEBUG_LBT# }"
            ;;
        DEBUG_GPS=*|\ DEBUG_GPS=*)
            DEBUG_GPS="${line#*=}"
            DEBUG_GPS="${DEBUG_GPS# }"
            ;;
        DEBUG_GPIO=*|\ DEBUG_GPIO=*)
            DEBUG_GPIO="${line#*=}"
            DEBUG_GPIO="${DEBUG_GPIO# }"
            ;;
        DEBUG_RAD=*|\ DEBUG_RAD=*)
            DEBUG_RAD="${line#*=}"
            DEBUG_RAD="${DEBUG_RAD# }"
            ;;
        DEBUG_CAL=*|\ DEBUG_CAL=*)
            DEBUG_CAL="${line#*=}"
            DEBUG_CAL="${DEBUG_CAL# }"
            ;;
        DEBUG_SX1302=*|\ DEBUG_SX1302=*)
            DEBUG_SX1302="${line#*=}"
            DEBUG_SX1302="${DEBUG_SX1302# }"
            ;;
        DEBUG_FTIME=*|\ DEBUG_FTIME=*)
            DEBUG_FTIME="${line#*=}"
            DEBUG_FTIME="${DEBUG_FTIME# }"
            ;;
    esac
done < "$LIBRARY_CFG"

cat > "$CONFIG_FILE" << EOF
#ifndef _LORAGW_CONFIGURATION_H
#define _LORAGW_CONFIGURATION_H
	#define LIBLORAGW_VERSION	"${LIBLORAGW_VERSION}"
	#define DEBUG_AUX		${DEBUG_AUX}
	#define DEBUG_COM		${DEBUG_COM}
	#define DEBUG_MCU		${DEBUG_MCU}
	#define DEBUG_I2C		${DEBUG_I2C}
	#define DEBUG_REG		${DEBUG_REG}
	#define DEBUG_HAL		${DEBUG_HAL}
	#define DEBUG_LBT		${DEBUG_LBT}
	#define DEBUG_GPS		${DEBUG_GPS}
	#define DEBUG_GPIO		${DEBUG_GPIO}
	#define DEBUG_RAD		${DEBUG_RAD}
	#define DEBUG_CAL		${DEBUG_CAL}
	#define DEBUG_SX1302	${DEBUG_SX1302}
	#define DEBUG_FTIME		${DEBUG_FTIME}
#endif
EOF

echo "Generated $CONFIG_FILE"
