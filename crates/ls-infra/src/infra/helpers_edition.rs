impl AppInfra {
    fn billing_feature_enabled(&self) -> bool {
        let config = self.config_snapshot();
        config.billing.enabled && config.edition_capabilities().billing_enabled
    }

    fn advanced_traffic_controls_enabled(&self) -> bool {
        self.config_snapshot()
            .edition_capabilities()
            .advanced_traffic_controls_enabled
    }
}
