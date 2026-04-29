#!/bin/sh
# Maverick SX1302 concentrator reset script
#
# Based on Semtech sx1302_hal reset_lgw.sh for CoreCell reference design.
# Resets the SX1302 chip and enables power/LDOs via GPIO.
#
# Usage:
#   maverick-reset-spi.sh start
#   maverick-reset-spi.sh stop

set -e

# GPIO mapping for CoreCell / RAK2287 reference design on Raspberry Pi
SX1302_RESET_PIN=23
SX1302_POWER_EN_PIN=18
SX1261_RESET_PIN=22
AD5338R_RESET_PIN=13

WAIT_GPIO() {
    sleep 0.1
}

init() {
    # Export GPIOs if not already exported
    if [ ! -d /sys/class/gpio/gpio${SX1302_RESET_PIN} ]; then
        echo "$SX1302_RESET_PIN" > /sys/class/gpio/export; WAIT_GPIO
    fi
    if [ ! -d /sys/class/gpio/gpio${SX1261_RESET_PIN} ]; then
        echo "$SX1261_RESET_PIN" > /sys/class/gpio/export; WAIT_GPIO
    fi
    if [ ! -d /sys/class/gpio/gpio${SX1302_POWER_EN_PIN} ]; then
        echo "$SX1302_POWER_EN_PIN" > /sys/class/gpio/export; WAIT_GPIO
    fi
    if [ ! -d /sys/class/gpio/gpio${AD5338R_RESET_PIN} ]; then
        echo "$AD5338R_RESET_PIN" > /sys/class/gpio/export; WAIT_GPIO
    fi

    # Set direction to output
    echo "out" > /sys/class/gpio/gpio${SX1302_RESET_PIN}/direction; WAIT_GPIO
    echo "out" > /sys/class/gpio/gpio${SX1261_RESET_PIN}/direction; WAIT_GPIO
    echo "out" > /sys/class/gpio/gpio${SX1302_POWER_EN_PIN}/direction; WAIT_GPIO
    echo "out" > /sys/class/gpio/gpio${AD5338R_RESET_PIN}/direction; WAIT_GPIO
}

reset() {
    # Power enable
    echo "1" > /sys/class/gpio/gpio${SX1302_POWER_EN_PIN}/value; WAIT_GPIO

    # SX1302 reset pulse
    echo "1" > /sys/class/gpio/gpio${SX1302_RESET_PIN}/value; WAIT_GPIO
    echo "0" > /sys/class/gpio/gpio${SX1302_RESET_PIN}/value; WAIT_GPIO

    # SX1261 reset
    echo "0" > /sys/class/gpio/gpio${SX1261_RESET_PIN}/value; WAIT_GPIO
    echo "1" > /sys/class/gpio/gpio${SX1261_RESET_PIN}/value; WAIT_GPIO

    # AD5338R reset
    echo "0" > /sys/class/gpio/gpio${AD5338R_RESET_PIN}/value; WAIT_GPIO
    echo "1" > /sys/class/gpio/gpio${AD5338R_RESET_PIN}/value; WAIT_GPIO
}

term() {
    # Unexport GPIOs
    if [ -d /sys/class/gpio/gpio${SX1302_RESET_PIN} ]; then
        echo "$SX1302_RESET_PIN" > /sys/class/gpio/unexport; WAIT_GPIO
    fi
    if [ -d /sys/class/gpio/gpio${SX1261_RESET_PIN} ]; then
        echo "$SX1261_RESET_PIN" > /sys/class/gpio/unexport; WAIT_GPIO
    fi
    if [ -d /sys/class/gpio/gpio${SX1302_POWER_EN_PIN} ]; then
        echo "$SX1302_POWER_EN_PIN" > /sys/class/gpio/unexport; WAIT_GPIO
    fi
    if [ -d /sys/class/gpio/gpio${AD5338R_RESET_PIN} ]; then
        echo "$AD5338R_RESET_PIN" > /sys/class/gpio/unexport; WAIT_GPIO
    fi
}

case "$1" in
    start)
        term 2>/dev/null || true
        init
        reset
        ;;
    stop)
        reset
        term
        ;;
    *)
        echo "Usage: $0 {start|stop}"
        exit 1
        ;;
esac

exit 0
