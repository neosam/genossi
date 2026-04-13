<?php

if ( ! defined( 'ABSPATH' ) ) {
    exit;
}

class Genossi_Member_Count_Settings {

    public static function init() {
        add_action( 'admin_menu', array( __CLASS__, 'add_menu' ) );
        add_action( 'admin_init', array( __CLASS__, 'register_settings' ) );
    }

    public static function add_menu() {
        add_options_page(
            'Genossi Member Count',
            'Genossi Member Count',
            'manage_options',
            'genossi-member-count',
            array( __CLASS__, 'render_page' )
        );
    }

    /**
     * Check if the API URL is provided by the genossi-beitritt plugin.
     *
     * @return bool True if genossi_api_url option is set and non-empty.
     */
    public static function has_beitritt_url() {
        $url = get_option( 'genossi_api_url', '' );
        return ! empty( $url );
    }

    public static function register_settings() {
        register_setting( 'genossi_mc_settings', 'genossi_mc_api_url', array(
            'type'              => 'string',
            'sanitize_callback' => array( __CLASS__, 'sanitize_api_url' ),
        ) );

        register_setting( 'genossi_mc_settings', 'genossi_mc_cache_ttl', array(
            'type'              => 'integer',
            'sanitize_callback' => array( __CLASS__, 'sanitize_cache_ttl' ),
            'default'           => 900,
        ) );

        add_settings_section(
            'genossi_mc_main',
            'API-Konfiguration',
            null,
            'genossi-member-count'
        );

        add_settings_field(
            'genossi_mc_api_url',
            'Genossi API-URL',
            array( __CLASS__, 'render_api_url_field' ),
            'genossi-member-count',
            'genossi_mc_main'
        );

        add_settings_field(
            'genossi_mc_cache_ttl',
            'Cache-Dauer (Sekunden)',
            array( __CLASS__, 'render_cache_ttl_field' ),
            'genossi-member-count',
            'genossi_mc_main'
        );
    }

    public static function sanitize_api_url( $value ) {
        $value = esc_url_raw( trim( $value ) );
        return rtrim( $value, '/' );
    }

    public static function sanitize_cache_ttl( $value ) {
        $value = (int) $value;
        if ( $value < 0 ) {
            $value = 0;
        }
        return $value;
    }

    public static function render_api_url_field() {
        if ( self::has_beitritt_url() ) {
            $beitritt_url = get_option( 'genossi_api_url', '' );
            echo '<input type="url" value="' . esc_attr( $beitritt_url ) . '" class="regular-text" disabled />';
            echo '<p class="description">Wird automatisch vom <strong>Genossi Beitritt</strong>-Plugin uebernommen.</p>';
        } else {
            $value = get_option( 'genossi_mc_api_url', '' );
            echo '<input type="url" name="genossi_mc_api_url" value="' . esc_attr( $value ) . '" class="regular-text" placeholder="https://genossi.example.com" />';
            echo '<p class="description">Basis-URL der Genossi-API (ohne /api/...). Wird automatisch uebernommen, wenn das Genossi-Beitritt-Plugin installiert ist.</p>';
        }
    }

    public static function render_cache_ttl_field() {
        $value = get_option( 'genossi_mc_cache_ttl', 900 );
        echo '<input type="number" name="genossi_mc_cache_ttl" value="' . esc_attr( $value ) . '" class="small-text" min="0" step="1" />';
        echo '<p class="description">Wie lange die Mitgliederzahl gecached wird. Standard: 900 (15 Minuten). 0 = kein Caching.</p>';
    }

    public static function render_page() {
        if ( ! current_user_can( 'manage_options' ) ) {
            return;
        }
        ?>
        <div class="wrap">
            <h1>Genossi Member Count Einstellungen</h1>
            <form method="post" action="options.php">
                <?php
                settings_fields( 'genossi_mc_settings' );
                do_settings_sections( 'genossi-member-count' );
                submit_button();
                ?>
            </form>
        </div>
        <?php
    }
}
