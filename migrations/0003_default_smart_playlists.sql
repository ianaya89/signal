-- Ship useful smart playlists out of the box (docs/03 §4 rule format).
INSERT INTO smart_playlists (name, rules, sort_order) VALUES
('Never Played',
 '{"match":"all","conditions":[{"field":"play_count","op":"eq","value":0}],"order_by":"added_at","order_dir":"desc","limit":null}',
 0),
('Recently Added',
 '{"match":"all","conditions":[{"field":"added_at","op":"within_days","value":30}],"order_by":"added_at","order_dir":"desc","limit":null}',
 1),
('24-bit Only',
 '{"match":"all","conditions":[{"field":"bit_depth","op":"gte","value":24}],"order_by":"sample_rate_hz","order_dir":"desc","limit":null}',
 2),
('Favorites',
 '{"match":"all","conditions":[{"field":"favorite","op":"eq","value":true}],"order_by":"added_at","order_dir":"desc","limit":null}',
 3),
('Most Played',
 '{"match":"all","conditions":[{"field":"play_count","op":"gt","value":0}],"order_by":"play_count","order_dir":"desc","limit":100}',
 4);
