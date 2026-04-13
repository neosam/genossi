<?php

if ( ! defined( 'ABSPATH' ) ) {
    exit;
}

class Genossi_Beitritt_Settings {

    public static function init() {
        add_action( 'admin_menu', array( __CLASS__, 'add_menu' ) );
        add_action( 'admin_init', array( __CLASS__, 'register_settings' ) );
    }

    public static function add_menu() {
        add_options_page(
            'Genossi Beitritt',
            'Genossi Beitritt',
            'manage_options',
            'genossi-beitritt',
            array( __CLASS__, 'render_page' )
        );
    }

    public static function register_settings() {
        register_setting( 'genossi_beitritt_settings', 'genossi_api_url', array(
            'type'              => 'string',
            'sanitize_callback' => array( __CLASS__, 'sanitize_api_url' ),
        ) );

        register_setting( 'genossi_beitritt_settings', 'genossi_api_key', array(
            'type'              => 'string',
            'sanitize_callback' => 'sanitize_text_field',
        ) );

        add_settings_section(
            'genossi_beitritt_main',
            'API-Konfiguration',
            null,
            'genossi-beitritt'
        );

        add_settings_field(
            'genossi_api_url',
            'Genossi API-URL',
            array( __CLASS__, 'render_api_url_field' ),
            'genossi-beitritt',
            'genossi_beitritt_main'
        );

        add_settings_field(
            'genossi_api_key',
            'API-Key',
            array( __CLASS__, 'render_api_key_field' ),
            'genossi-beitritt',
            'genossi_beitritt_main'
        );
    }

    public static function sanitize_api_url( $value ) {
        $value = esc_url_raw( trim( $value ) );
        if ( empty( $value ) ) {
            add_settings_error(
                'genossi_api_url',
                'genossi_api_url_empty',
                'Die API-URL darf nicht leer sein.'
            );
        }
        return rtrim( $value, '/' );
    }

    public static function render_api_url_field() {
        $value = get_option( 'genossi_api_url', '' );
        echo '<input type="url" name="genossi_api_url" value="' . esc_attr( $value ) . '" class="regular-text" placeholder="https://genossi.example.com" required />';
        echo '<p class="description">Basis-URL der Genossi-API (ohne /api/...)</p>';
    }

    public static function render_api_key_field() {
        $value = get_option( 'genossi_api_key', '' );
        echo '<input type="text" name="genossi_api_key" value="' . esc_attr( $value ) . '" class="regular-text" required />';
        echo '<p class="description">API-Key fuer den Zugriff auf die Public-Join-API</p>';
    }

    public static function render_page() {
        if ( ! current_user_can( 'manage_options' ) ) {
            return;
        }
        ?>
        <div class="wrap">
            <h1>Genossi Beitritt Einstellungen</h1>
            <form method="post" action="options.php">
                <?php
                settings_fields( 'genossi_beitritt_settings' );
                do_settings_sections( 'genossi-beitritt' );
                submit_button();
                ?>
            </form>
        </div>
        <?php
    }
}
