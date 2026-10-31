# M3g9 raw performance observations

The `before/` and `after/` directories retain all three individual samples, medians, RSS, outliers, allocation calls/reallocations/requested bytes and tool-specific methods for16 matched frozen tools. `pickups.json` separately records17 new workloads in both production and diagnostic modes. `followup.json` retains the interleaved investigation of three full-batch timing outliers. These are host observations, not hard timing gates or universal performance guarantees.

Both matched batches used Rust1.99.1 and identical frozen tool hashes/workloads. Before is the clean merged baseline; after is the clean runtime checkpoint named in its provenance. Subsequent changes affect documentation, evidence and observer formatting; the measured gameplay implementation is unchanged. No local source/test/fuzz process overlapped either batch. Shared-host scheduling, allocation layout and separate process/build paths still limit interpretation. New benchmark continuations use default commands after short source scenarios end; they are not additional C coverage. Whole-process RSS includes source evidence parsing/setup; interval allocation bytes are cumulative requests, not peak live heap or leak measurements.

247 matched workload groups retain40 geometry/service groups and207 digest/allocation groups. All207 digests and three-sample allocation arrays match exactly;45 transaction allocation/copy observations also match. Timing/RSS variation remains visible.

## Representative matched production advances

Values are milliseconds per100 advances. All individual samples for every other group remain in the linked JSON files.

| Workload | Before samples | After samples | Before/after median | RSS medians KiB |
| --- | --- | --- | --- | --- |
|small_control (17)|10.823/10.644/10.723|11.435/11.344/11.296|10.723/11.344|3008/2824|
|small_control (20)|11.002/11.181/10.797|10.924/10.635/10.597|11.002/10.635|2912/2908|
|long_lifecycle (20)|11.144/10.810/11.149|10.537/10.702/10.650|11.144/10.650|3680/3584|
|four_owners (20)|46.762/48.105/46.985|45.727/46.390/46.762|46.985/46.390|3788/3728|
|wide_map_lifecycle (20)|11.677/12.013/12.086|11.311/11.800/11.331|12.013/11.331|3844/3728|
|expanded_sight_dag (20)|11.387/11.099/11.116|10.645/10.709/10.885|11.116/10.709|3120/2960|
|line_table (20)|11.623/11.494/11.862|10.787/11.736/10.860|11.623/10.860|3504/3672|
|high_cell_demon (20)|11.933/13.124/11.962|13.059/11.552/11.884|11.962/11.884|4400/4352|
|canonical_small (original)|10.798/11.013/10.888|10.569/10.267/10.974|10.888/10.569|2616/2568|
|actor_capacity (original)|110.762/102.598/103.941|103.713/97.820/111.614|103.941/103.713|3076/2920|

## New pickup production intervals

Full encounters run512 advances and normalize timing to100; their allocation counters cover all512. Other intervals run100. Labels describe continuous source scenarios, including setup/warmup declared in `pickups.json`, rather than isolated action costs. Initialization/source parsing/final Snapshot are outside the timer/counters and inside whole-process RSS.

| Workload | Three ms/100 samples | Median | RSS median KiB | Calls/reallocations/requested bytes |
| --- | --- | --- | --- | --- |
|idle|1.059/1.064/1.106|1.064|35736|28100/200/1709800|
|clip|1.051/1.044/1.041|1.044|35736|28198/204/1652352|
|full_ammo|1.056/1.123/1.049|1.056|35752|28198/234/1813296|
|four_ammo|1.047/1.118/1.112|1.112|35768|28198/211/1758648|
|stim|1.041/1.009/1.012|1.012|35772|28198/204/1646424|
|medikit|1.091/1.091/1.049|1.091|35772|28198/204/1646424|
|blocked_pickup|1.060/1.055/1.094|1.060|35736|28330/249/1705888|
|sixteen|1.297/1.275/1.310|1.297|35852|34454/233/2400384|
|shotgun|6.286/6.629/6.224|6.286|35768|185350/390/8277848|
|chaingun|6.141/6.402/6.245|6.245|35768|182718/450/8251733|
|world_weapon_fight|6.517/6.330/6.534|6.517|35768|186427/436/8715080|
|supplies_melee|6.124/6.016/6.264|6.124|35760|176558/315/7571266|
|after_heal|7.031/6.475/6.449|6.475|35908|188724/367/8104083|
|player_death|6.307/5.845/5.691|5.845|35772|170398/264/7277255|
|death_transition|5.167/4.774/4.743|4.774|35852|139989/250/5992585|
|full_encounter|5.696/5.597/5.712|5.696|35680|848691/1733/37162144|
|losing_encounter|5.817/5.753/6.177|5.817|35764|876649/1477/37978488|

## Timing outlier investigation

The full batch showed+56.9% pursuit diagnostic-step,+35.5% player-body demon-source diagnostic-step and+32.7% continuous-command advance medians. Their after samples contained two slow observations. A quiet interleaved follow-up rebuilt the exact baseline in a detached worktree with matching compiler/settings and alternated process order; it did not reproduce those magnitudes. This does not prove universal equivalence or a speedup. Original outliers remain in the raw batch.

| Workload | Follow-up before ms samples | Follow-up after ms samples | Median change |
| --- | --- | --- | --- |
|player_demon pursuit step|6.279/6.344/6.367|6.241/6.449/6.252|-1.45%|
|original_player demon_source step|1.979/2.001/2.104|1.978/1.975/2.074|-1.13%|
|player_commands continuous advance|1.109/1.067/1.170|1.152/1.129/1.012|+1.80%|

Raw records: [matched comparison](comparison.json), [new intervals](pickups.json), [interleaved follow-up](followup.json), [before provenance](before/provenance.json), [after provenance](after/provenance.json).
