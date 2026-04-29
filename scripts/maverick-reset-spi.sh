#!/bin/sh
# Maverick SX1302 concentrator reset script
#
# Based on Semtech sx1302_hal reset_lgw.sh for CoreCell reference design.
# Updated for RAK2287 Pi HAT compatibility and kernel 6.12+ GPIO numbering.
#
# RAK2287 uses GPIO 17 for SX1302 reset (active HIGH logic observed).
# Kernel 6.12+ sysfs GPIO uses gpiochip0 base offset 512.
#
# Usage:
#   maverick-reset-spi.sh start
#   maverick-reset-spi.sh stop

set -e

# GPIO mapping for RAK2287 on Raspberry Pi (kernel 6.12+ numbering)
# gpiochip0 base = 512
GPIO_BASE=512
SX1302_RESET_PIN=$((GPIO_BASE + 17))   # GPIO 17 -> SX1302 reset
SX1302_POWER_EN_PIN=$((GPIO_BASE + 18)) # GPIO 18 -> SX1302 power enable

WAIT_GPIO() {
    sleep 0.1
}

init() {
    # Export GPIOs if not already exported
    if [ ! -d /sys/class/gpio/gpio${SX1302_RESET_PIN} ]; then
        echo "$SX1302_RESET_PIN" > /sys/class/gpio/export; WAIT_GPIO
    fi
    if [ ! -d /sys/class/gpio/gpio${SX1302_POWER_EN_PIN} ]; then
        echo "$SX1302_POWER_EN_PIN" > /sys/class/gpio/export; WAIT_GPIO
    fi

    # Set direction to output
    echo "out" > /sys/class/gpio/gpio${SX1302_RESET_PIN}/direction; WAIT_GPIO
    echo "out" > /sys/class/gpio/gpio${SX1302_POWER_EN_PIN}/direction; WAIT_GPIO
}

reset() {
    # Power enable
    echo "1" > /sys/class/gpio/gpio${SX1302_POWER_EN_PIN}/value; WAIT_GPIO

    # SX1302 reset pulse: HIGH 1s then LOW (active HIGH reset observed on RAK2287)
    echo "1" > /sys/class/gpio/gpio${SX1302_RESET_PIN}/value
    sleep 1
    echo "0" > /sys/class/gpio/gpio${SX1302_RESET_PIN}/value; WAIT_GPIO
}

term() {
    # Unexport GPIOs
    if [ -d /sys/class/gpio/gpio${SX1302_RESET_PIN} ]; then
        echo "$SX1302_RESET_PIN" > /sys/class/gpio/unexport; WAIT_GPIO
    fi
    if [ -d /sys/class/gpio/gpio${SX1302_POWER_EN_PIN} ]; then
        echo "$SX1302_POWER_EN_PIN" > /sys/class/gpio/unexport; WAIT_GPIO
    fi
}

case "$1" in
    start)
        term 2>/dev/null || true
        init
        reset
        # Wait for concentrator to stabilize after reset
        sleep 2
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
