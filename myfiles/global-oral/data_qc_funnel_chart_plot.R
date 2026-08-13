library(tidyverse)
library(scales)

# 1. Global theme & font sizing
base_sz <- 14
text_sz <- base_sz / .pt  

update_geom_defaults("text", list(family = "sans"))
update_geom_defaults("label", list(family = "sans"))

theme_set(theme_minimal(base_size = base_sz, base_family = "sans"))

# 2. Funnel Data (Main Nodes)
funnel_data <- data.frame(
  step     = 1:5,
  n_num    = c(114959, 113233, 108691, 80788, 51186),
  s_num    = c(334, 312, 288, 288, 288),
  y        = c(16.0, 12.0, 8.0, 4.0, 0.0)
)

funnel_data <- funnel_data %>%
  mutate(
    pct = (n_num / n_num[1]) * 100,
    n_inside = comma(n_num),
    s_inside = case_when(
      step == 1 ~ "Raw Merge (DADA2-processed, CR > 0)\n334 studies",
      step == 2 ~ sprintf("Retained: %.1f%%\n312 studies", pct),
      step == 3 ~ sprintf("Retained: %.1f%%\n288 studies", pct),
      step == 4 ~ sprintf("Retained: %.1f%%\n288 studies", pct),
      step == 5 ~ "Final Curated Dataset\n42,224 subjects | 61,235 features"
    )
  )

# 3. Geometry Calculations for Funnel Blocks
max_w <- 7.5
min_w <- 4.5
bar_h <- 0.90

funnel_data$half_w <- min_w +
  (funnel_data$n_num - min(funnel_data$n_num)) /
  (max(funnel_data$n_num) - min(funnel_data$n_num)) * (max_w - min_w)

poly_blocks <- do.call(rbind, lapply(1:nrow(funnel_data), function(i) {
  b <- funnel_data[i, ]
  hw <- b$half_w
  yc <- b$y
  data.frame(
    x    = c(-hw, hw, hw, -hw),
    y    = c(yc - bar_h, yc - bar_h, yc + bar_h, yc + bar_h),
    step = b$step,
    grp  = paste0("bar_", i)
  )
}))

conn_indices <- 1:4
conn_blocks <- do.call(rbind, lapply(conn_indices, function(i) {
  top_hw      <- funnel_data$half_w[i]
  bot_hw      <- funnel_data$half_w[i + 1]
  top_y_bottom <- funnel_data$y[i] - bar_h
  bot_y_top   <- funnel_data$y[i + 1] + bar_h
  data.frame(
    x    = c(-top_hw, top_hw, bot_hw, -bot_hw),
    y    = c(top_y_bottom, top_y_bottom, bot_y_top, bot_y_top),
    step = i,
    grp  = paste0("conn_", i)
  )
}))

# 4. Exclusion Branches (Right-side Drops)
drops <- data.frame(
  step = 1:4,
  y_mid = (funnel_data$y[1:4] + funnel_data$y[2:5]) / 2, 
  title = c("Step 1: Chimera Filter",
            "Step 2: CR Mapping Filter",
            "Step 3: Site Filter",
            "Step 4: Deduplication"),
  criteria = c(">30% oral samples with chimera >25%",
               "Oral aggregate GG2 CR mapping <70%",
               "Remove non-oral samples",
               "1 sample per (subject_id \u00D7 site)"),
  ex_samp = c(1726, 4542, 27903, 29602),
  ex_stud = c(22, 24, 0, 0)
)

drops <- drops %>%
  mutate(
    x_start = (funnel_data$half_w[1:4] + funnel_data$half_w[2:5]) / 2,
    x_end   = max_w + 1.5, 
    ex_text = ifelse(ex_stud > 0,
                     sprintf("\u2212%s samples, \u2212%s studies", comma(ex_samp), ex_stud),
                     sprintf("\u2212%s samples", comma(ex_samp)))
  )

# 5. Plot Generation
bar_fills <- c("#1A237E", "#283593", "#3949AB", "#1E88E5", "#43A047")

p <- ggplot() +
  geom_segment(aes(x = -max_w - 0.8, xend = -max_w - 0.8, y = 17, yend = 3), 
               color = "#B0BEC5", linewidth = 0.8) +
  geom_segment(aes(x = -max_w - 0.8, xend = -max_w - 0.5, y = 17, yend = 17), 
               color = "#B0BEC5", linewidth = 0.8) +
  geom_segment(aes(x = -max_w - 0.8, xend = -max_w - 0.5, y = 3, yend = 3), 
               color = "#B0BEC5", linewidth = 0.8) +
  annotate("text", x = -max_w - 1.3, y = 10, label = "Dataset-Level Filters", 
           angle = 90, fontface = "bold", color = "#546E7A", size = text_sz * 1.1) +

  geom_polygon(data = conn_blocks, aes(x = x, y = y, group = grp),
               fill = "#ECEFF1", alpha = 0.8) +
  geom_segment(data = drops, aes(x = 0, xend = 0, y = y_mid + 1.2, yend = y_mid - 1.2),
               arrow = arrow(length = unit(0.2, "cm"), type = "closed"), 
               color = "#90A4AE", linewidth = 0.8) +
  geom_polygon(data = poly_blocks, aes(x = x, y = y, group = grp),
               fill = rep(bar_fills, each = 4), color = NA) +
  
  geom_text(data = funnel_data, aes(x = 0, y = y + 0.40, label = n_inside),
            size = text_sz * 1.2, color = "white", fontface = "bold") +
  geom_text(data = funnel_data, aes(x = 0, y = y - 0.40, label = s_inside),
            size = text_sz * 0.8, color = "grey90", lineheight = 1.0) +

  geom_segment(data = drops, aes(x = x_start, xend = x_end, y = y_mid, yend = y_mid),
               arrow = arrow(length = unit(0.2, "cm"), type = "closed"), 
               color = "#E53935", linewidth = 0.7) +
  geom_text(data = drops, aes(x = x_end + 0.5, y = y_mid + 0.6, label = title), 
            hjust = 0, fontface = "bold", color = "#212121", size = text_sz * 0.95) +
  geom_text(data = drops, aes(x = x_end + 0.5, y = y_mid, label = criteria), 
            hjust = 0, fontface = "italic", color = "#757575", size = text_sz * 0.85) +
  geom_text(data = drops, aes(x = x_end + 0.5, y = y_mid - 0.6, label = ex_text), 
            hjust = 0, fontface = "bold", color = "#D32F2F", size = text_sz * 0.9) +

  coord_cartesian(xlim = c(-10.0, 22.0), ylim = c(-2.0, 18.0), clip = "off") +
  theme_void(base_size = base_sz, base_family = "sans") +
  theme(plot.margin = margin(20, 10, 20, 10, unit = "pt"))

# 6. Save Output
output_dir <- "/home/yanglun/YSD/global_oral/global_oral_v2/oral-v2-02/01-overview-sampling-distribution/02-patch-map-2/02-patch-pipeline-map/output"
if (!dir.exists(output_dir)) dir.create(output_dir, recursive = TRUE)

ggsave(file.path(output_dir, "data_qc_funnel_chart_hybrid.pdf"),
       plot = p, width = 13.5, height = 9, device = "pdf", useDingbats = FALSE)
ggsave(file.path(output_dir, "data_qc_funnel_chart_hybrid.png"),
       plot = p, width = 13.5, height = 9, device = "png", dpi = 600, bg = "white")

message("Hybrid QC funnel chart saved to: ", file.path(output_dir, "data_qc_funnel_chart_hybrid.pdf"))