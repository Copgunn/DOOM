# M3h6 IWAD geometry counts

Output of `python3 tools/iwad_counts.py --markdown` over the owner's two classic IWADs
(68 map slots), read locally and read-only on 2026-11-02. The table holds counts and
pass/fail facts only: a map is named by its slot, no lump name beyond the slot and no
coordinate of any object appears (spans are differences). The rules are those of the
[M3h6 investigation](../M3H6_REFERENCE_INVESTIGATION.md): the arithmetic certificate
(i)–(vii) of §4.2, the boundary rules C1–C3 (C3 is `n/t`, not tested, where C1 or C2
fails), the closed-cell BLOCKMAP rule, grid coverage and REJECT length of §4.5, and the
2 × 2 slide-window bound of §4.4. "Geometry admitted" is the conjunction of every rule
and says nothing about THINGs, specials or sky, which are rejected separately
([§9](../M3H6_REFERENCE_INVESTIGATION.md#9-e1m1-readiness-report)). The script is
local-only and is never run by CI.

| Slot | Vertices | Linedefs | Sidedefs | Sectors | THINGs | Segs | Subsectors | Nodes | Cells | Words | Max lines/cell | Span x | Span y | L1 | Max height | 2x2 window | C1 | C2 odd | C3 | Missing listings | REJECT | (i) | (ii) | (iii) | (iv) | (v) | (vi) | (vii) | Boundary | Geometry admitted |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| E1M1 | 470 | 486 | 666 | 88 | 143 | 747 | 239 | 238 | 828 | 3474 | 13 | 4576 | 2816 | 832 | 264 | 28 | 0 | 0 | 0 | 0 | pass | pass | pass | pass | pass | pass | pass | pass | pass | pass |
| E1M2 | 942 | 1033 | 1323 | 200 | 262 | 1463 | 448 | 447 | 1302 | 5935 | 20 | 5312 | 3904 | 2320 | 512 | 31 | 0 | 0 | 0 | 0 | pass | pass | FAIL | pass | pass | pass | pass | pass | pass | FAIL |
| E1M3 | 946 | 1026 | 1326 | 177 | 380 | 1445 | 461 | 460 | 850 | 4447 | 22 | 4224 | 3056 | 704 | 528 | 41 | 1 | 2 | n/t | 0 | pass | pass | pass | pass | pass | pass | pass | FAIL | FAIL | FAIL |
| E1M4 | 780 | 830 | 1054 | 139 | 254 | 1172 | 355 | 354 | 660 | 3609 | 16 | 3720 | 2784 | 1024 | 400 | 30 | 0 | 0 | 0 | 0 | pass | pass | pass | pass | pass | pass | pass | pass | pass | pass |
| E1M5 | 746 | 825 | 1053 | 143 | 293 | 1141 | 384 | 383 | 832 | 4016 | 23 | 3968 | 3232 | 896 | 248 | 44 | 0 | 0 | 0 | 0 | pass | pass | pass | pass | pass | pass | pass | pass | pass | pass |
| E1M6 | 1207 | 1352 | 1727 | 250 | 463 | 1862 | 606 | 605 | 1748 | 7902 | 26 | 5776 | 4768 | 1056 | 304 | 56 | 0 | 0 | 0 | 0 | pass | pass | pass | pass | pass | pass | pass | pass | pass | pass |
| E1M7 | 896 | 958 | 1223 | 170 | 358 | 1371 | 467 | 466 | 864 | 4423 | 23 | 4032 | 3432 | 1088 | 320 | 39 | 0 | 0 | 0 | 0 | pass | pass | pass | pass | pass | pass | pass | pass | pass | pass |
| E1M8 | 328 | 333 | 511 | 74 | 126 | 586 | 177 | 176 | 2912 | 9700 | 16 | 6616 | 7136 | 5768 | 240 | 31 | 0 | 0 | 0 | 0 | pass | FAIL | FAIL | pass | pass | pass | pass | pass | pass | FAIL |
| E1M9 | 581 | 653 | 902 | 147 | 237 | 978 | 288 | 287 | 702 | 3390 | 12 | 3392 | 3200 | 896 | 304 | 25 | 0 | 0 | 0 | 0 | pass | pass | pass | pass | pass | pass | pass | pass | pass | pass |
| E2M1 | 371 | 415 | 550 | 92 | 127 | 597 | 180 | 179 | 336 | 1853 | 17 | 2576 | 1984 | 896 | 384 | 30 | 0 | 0 | 0 | 0 | pass | pass | pass | pass | pass | pass | pass | pass | pass | pass |
| E2M2 | 1537 | 1610 | 2076 | 245 | 311 | 2268 | 725 | 724 | 1066 | 6191 | 20 | 3200 | 5184 | 1824 | 320 | 43 | 0 | 0 | 0 | 0 | pass | pass | FAIL | pass | pass | pass | pass | pass | pass | FAIL |
| E2M3 | 874 | 953 | 1145 | 128 | 229 | 1209 | 315 | 314 | 560 | 3343 | 19 | 3552 | 2520 | 608 | 256 | 39 | 0 | 0 | 0 | 0 | pass | pass | pass | pass | pass | pass | pass | pass | pass | pass |
| E2M4 | 1054 | 1117 | 1439 | 205 | 265 | 1589 | 497 | 496 | 930 | 5059 | 20 | 3904 | 3776 | 1408 | 624 | 47 | 0 | 0 | 0 | 0 | pass | pass | pass | pass | pass | pass | pass | FAIL | pass | FAIL |
| E2M5 | 1235 | 1353 | 1710 | 241 | 326 | 1847 | 620 | 619 | 1020 | 5581 | 19 | 4288 | 3712 | 1344 | 320 | 34 | 0 | 0 | 0 | 0 | pass | pass | pass | pass | pass | pass | pass | pass | pass | pass |
| E2M6 | 967 | 1094 | 1324 | 181 | 326 | 1409 | 460 | 459 | 1040 | 5230 | 28 | 3296 | 5000 | 1696 | 288 | 48 | 0 | 0 | 0 | 0 | pass | pass | pass | pass | pass | pass | pass | pass | pass | pass |
| E2M7 | 1626 | 1764 | 2252 | 323 | 235 | 2438 | 818 | 817 | 736 | 5124 | 24 | 3968 | 2880 | 1216 | 384 | 48 | 0 | 0 | 0 | 0 | pass | pass | pass | pass | pass | pass | pass | pass | pass | pass |
| E2M8 | 216 | 218 | 278 | 27 | 66 | 310 | 130 | 129 | 1000 | 3582 | 15 | 3136 | 5056 | 3136 | 280 | 32 | 0 | 0 | 0 | 0 | pass | pass | FAIL | pass | pass | pass | pass | pass | pass | FAIL |
| E2M9 | 160 | 166 | 198 | 19 | 53 | 209 | 68 | 67 | 464 | 1788 | 15 | 3584 | 1920 | 704 | 168 | 40 | 0 | 0 | 0 | 0 | pass | pass | pass | pass | pass | pass | pass | pass | pass | pass |
| E3M1 | 194 | 208 | 274 | 39 | 92 | 298 | 94 | 93 | 665 | 2528 | 15 | 2304 | 4352 | 1408 | 192 | 22 | 0 | 0 | 0 | 0 | pass | pass | pass | pass | pass | pass | pass | pass | pass | pass |
| E3M2 | 531 | 494 | 625 | 68 | 208 | 734 | 287 | 286 | 805 | 3492 | 15 | 2872 | 4352 | 1720 | 272 | 29 | 0 | 0 | 0 | 0 | pass | pass | pass | pass | pass | pass | pass | pass | pass | pass |
| E3M3 | 986 | 1102 | 1394 | 192 | 209 | 1478 | 441 | 440 | 672 | 3877 | 36 | 3520 | 3008 | 536 | 360 | 47 | 0 | 0 | 0 | 0 | pass | pass | pass | pass | pass | pass | pass | pass | pass | pass |
| E3M4 | 1027 | 1099 | 1369 | 186 | 374 | 1498 | 474 | 473 | 990 | 4901 | 22 | 4160 | 3800 | 1504 | 472 | 49 | 0 | 0 | 0 | 0 | pass | pass | FAIL | pass | pass | pass | pass | pass | pass | FAIL |
| E3M5 | 1222 | 1416 | 1846 | 268 | 291 | 1937 | 560 | 559 | 1406 | 6566 | 33 | 4800 | 4624 | 704 | 384 | 69 | 0 | 0 | 0 | 0 | pass | pass | pass | pass | pass | pass | pass | pass | pass | pass |
| E3M6 | 691 | 687 | 949 | 125 | 335 | 1108 | 410 | 409 | 2916 | 10709 | 12 | 6784 | 6792 | 5928 | 256 | 35 | 0 | 0 | 0 | 0 | pass | FAIL | FAIL | pass | pass | pass | pass | pass | pass | FAIL |
| E3M7 | 884 | 982 | 1247 | 166 | 154 | 1329 | 474 | 473 | 930 | 4832 | 16 | 3712 | 3840 | 1472 | 256 | 29 | 0 | 0 | 0 | 0 | pass | pass | pass | pass | pass | pass | pass | pass | pass | pass |
| E3M8 | 135 | 122 | 173 | 17 | 58 | 204 | 78 | 77 | 960 | 3404 | 10 | 3712 | 3992 | 1536 | 256 | 16 | 0 | 0 | 0 | 0 | pass | pass | pass | pass | pass | pass | pass | pass | pass | pass |
| E3M9 | 456 | 474 | 589 | 71 | 197 | 649 | 213 | 212 | 2070 | 7389 | 19 | 5640 | 5824 | 2120 | 184 | 29 | 0 | 0 | 0 | 0 | pass | pass | FAIL | pass | pass | pass | pass | pass | pass | FAIL |
| E4M1 | 439 | 471 | 642 | 81 | 141 | 695 | 196 | 195 | 162 | 1220 | 23 | 2176 | 1088 | 640 | 152 | 52 | 2 | 8 | n/t | 0 | pass | pass | pass | pass | pass | pass | pass | pass | FAIL | FAIL |
| E4M2 | 611 | 685 | 1014 | 122 | 227 | 1080 | 400 | 399 | 378 | 2227 | 18 | 2200 | 2648 | 384 | 440 | 45 | 0 | 0 | 0 | 0 | pass | pass | pass | pass | pass | pass | pass | pass | pass | pass |
| E4M3 | 784 | 822 | 1216 | 156 | 384 | 1359 | 454 | 453 | 1330 | 5669 | 25 | 4800 | 4416 | 1464 | 304 | 47 | 0 | 6 | n/t | 0 | pass | pass | FAIL | pass | pass | pass | pass | pass | FAIL | FAIL |
| E4M4 | 580 | 648 | 925 | 123 | 153 | 969 | 314 | 313 | 460 | 2398 | 17 | 2832 | 2496 | 512 | 224 | 44 | 0 | 0 | 0 | 0 | pass | pass | pass | pass | pass | pass | pass | pass | pass | pass |
| E4M5 | 1044 | 1068 | 1705 | 170 | 267 | 1902 | 674 | 673 | 896 | 4662 | 21 | 4000 | 3440 | 1376 | 448 | 44 | 0 | 0 | 0 | 0 | pass | pass | pass | pass | pass | pass | pass | pass | pass | pass |
| E4M6 | 1315 | 1355 | 1963 | 224 | 295 | 2172 | 773 | 772 | 805 | 4535 | 24 | 4392 | 2888 | 1152 | 304 | 62 | 0 | 0 | 0 | 0 | pass | pass | pass | pass | pass | pass | pass | pass | pass | pass |
| E4M7 | 1273 | 1371 | 1981 | 288 | 258 | 2155 | 800 | 799 | 775 | 4539 | 22 | 3072 | 3872 | 640 | 264 | 48 | 0 | 0 | 0 | 8 | pass | pass | pass | pass | pass | pass | pass | pass | pass | FAIL |
| E4M8 | 1158 | 1280 | 1908 | 223 | 315 | 2043 | 726 | 725 | 1044 | 5196 | 19 | 3680 | 4552 | 832 | 336 | 56 | 0 | 2 | n/t | 0 | pass | pass | pass | pass | pass | pass | pass | pass | FAIL | FAIL |
| E4M9 | 1565 | 1644 | 2417 | 248 | 247 | 2671 | 956 | 955 | 684 | 4655 | 28 | 2296 | 4576 | 800 | 384 | 89 | 0 | 0 | 0 | 0 | pass | pass | pass | pass | pass | pass | pass | pass | pass | pass |
| MAP01 | 383 | 370 | 529 | 59 | 69 | 601 | 194 | 193 | 783 | 3209 | 17 | 3376 | 3648 | 1656 | 264 | 29 | 0 | 0 | 0 | 0 | pass | pass | pass | pass | pass | pass | pass | pass | pass | pass |
| MAP02 | 501 | 523 | 715 | 97 | 173 | 797 | 224 | 223 | 240 | 1567 | 23 | 1888 | 1904 | 752 | 176 | 47 | 1 | 2 | n/t | 0 | pass | pass | pass | pass | pass | pass | pass | pass | FAIL | FAIL |
| MAP03 | 544 | 594 | 894 | 121 | 148 | 980 | 301 | 300 | 384 | 2172 | 18 | 2984 | 1984 | 920 | 264 | 37 | 0 | 0 | 0 | 0 | pass | pass | pass | pass | pass | pass | pass | pass | pass | pass |
| MAP04 | 494 | 591 | 780 | 122 | 138 | 812 | 249 | 248 | 238 | 1635 | 22 | 2056 | 1664 | 384 | 168 | 49 | 0 | 0 | 0 | 0 | pass | pass | pass | pass | pass | pass | pass | pass | pass | pass |
| MAP05 | 864 | 958 | 1291 | 176 | 217 | 1391 | 409 | 408 | 425 | 2847 | 26 | 3136 | 2112 | 712 | 320 | 66 | 0 | 0 | 0 | 0 | pass | pass | pass | pass | pass | pass | pass | pass | pass | pass |
| MAP06 | 935 | 1011 | 1409 | 183 | 247 | 1551 | 484 | 483 | 630 | 3658 | 19 | 3808 | 2560 | 1088 | 568 | 46 | 0 | 0 | 0 | 0 | pass | pass | pass | pass | pass | pass | pass | FAIL | pass | FAIL |
| MAP07 | 195 | 188 | 283 | 29 | 98 | 325 | 97 | 96 | 304 | 1412 | 12 | 2368 | 1984 | 2176 | 128 | 21 | 0 | 0 | 0 | 0 | pass | pass | pass | pass | pass | pass | pass | pass | pass | pass |
| MAP08 | 574 | 649 | 868 | 127 | 242 | 919 | 276 | 275 | 930 | 4132 | 17 | 3904 | 3776 | 1024 | 224 | 32 | 0 | 0 | 0 | 0 | pass | pass | pass | pass | pass | pass | pass | pass | pass | pass |
| MAP09 | 617 | 640 | 877 | 131 | 392 | 992 | 325 | 324 | 1088 | 4725 | 16 | 4312 | 4048 | 1344 | 448 | 28 | 0 | 0 | 0 | 0 | pass | pass | pass | pass | pass | pass | pass | pass | pass | pass |
| MAP10 | 961 | 1054 | 1312 | 186 | 509 | 1417 | 455 | 454 | 1122 | 5477 | 19 | 4192 | 4224 | 1216 | 304 | 31 | 0 | 0 | 0 | 0 | pass | pass | pass | pass | pass | pass | pass | pass | pass | pass |
| MAP11 | 780 | 850 | 1195 | 143 | 238 | 1309 | 430 | 429 | 667 | 3475 | 19 | 3584 | 2864 | 672 | 544 | 40 | 0 | 0 | 0 | 0 | pass | pass | pass | pass | pass | pass | pass | FAIL | pass | FAIL |
| MAP12 | 774 | 774 | 1094 | 162 | 195 | 1277 | 423 | 422 | 1085 | 5146 | 21 | 3904 | 4416 | 4416 | 296 | 46 | 0 | 0 | 0 | 0 | pass | FAIL | FAIL | pass | pass | pass | pass | pass | pass | FAIL |
| MAP13 | 1103 | 1196 | 1670 | 247 | 312 | 1837 | 557 | 556 | 925 | 5045 | 29 | 3168 | 4672 | 3648 | 640 | 67 | 0 | 0 | 0 | 0 | pass | pass | FAIL | pass | pass | pass | pass | FAIL | pass | FAIL |
| MAP14 | 1429 | 1681 | 2587 | 348 | 173 | 2815 | 851 | 850 | 868 | 5325 | 54 | 3904 | 3520 | 1600 | 296 | 101 | 0 | 12 | n/t | 0 | pass | pass | FAIL | pass | pass | pass | pass | pass | FAIL | FAIL |
| MAP15 | 1601 | 1690 | 2361 | 301 | 376 | 2647 | 875 | 874 | 2030 | 9226 | 19 | 4384 | 7320 | 2040 | 600 | 50 | 0 | 0 | 0 | 0 | pass | pass | FAIL | pass | pass | pass | pass | FAIL | pass | FAIL |
| MAP16 | 754 | 778 | 1088 | 142 | 249 | 1230 | 421 | 420 | 1482 | 6135 | 23 | 4800 | 4864 | 2920 | 264 | 40 | 0 | 2 | n/t | 0 | pass | pass | FAIL | pass | pass | pass | pass | pass | FAIL | FAIL |
| MAP17 | 1107 | 1290 | 1847 | 274 | 421 | 1980 | 615 | 614 | 729 | 4319 | 25 | 3360 | 3336 | 576 | 728 | 60 | 0 | 2 | n/t | 0 | pass | pass | pass | pass | pass | pass | pass | FAIL | FAIL | FAIL |
| MAP18 | 806 | 774 | 986 | 108 | 498 | 1132 | 416 | 415 | 1440 | 6011 | 18 | 4992 | 4504 | 2496 | 352 | 26 | 0 | 4 | n/t | 0 | pass | pass | FAIL | pass | pass | pass | pass | pass | FAIL | FAIL |
| MAP19 | 1252 | 1255 | 1821 | 251 | 437 | 2117 | 721 | 720 | 2112 | 9300 | 19 | 5568 | 6016 | 5568 | 368 | 43 | 0 | 0 | 1 | 0 | pass | FAIL | FAIL | pass | pass | pass | pass | pass | FAIL | FAIL |
| MAP20 | 1024 | 1015 | 1448 | 191 | 256 | 1678 | 577 | 576 | 2050 | 8580 | 18 | 5184 | 6288 | 3840 | 648 | 41 | 0 | 0 | 0 | 0 | pass | pass | FAIL | pass | pass | pass | pass | FAIL | pass | FAIL |
| MAP21 | 396 | 327 | 538 | 61 | 217 | 640 | 229 | 228 | 672 | 2884 | 15 | 3488 | 2952 | 1280 | 256 | 23 | 0 | 0 | 0 | 0 | pass | pass | pass | pass | pass | pass | pass | pass | pass | pass |
| MAP22 | 590 | 657 | 940 | 140 | 121 | 995 | 316 | 315 | 285 | 1920 | 36 | 2320 | 1888 | 448 | 256 | 72 | 0 | 12 | n/t | 0 | pass | pass | pass | pass | pass | pass | pass | pass | FAIL | FAIL |
| MAP23 | 394 | 391 | 518 | 69 | 431 | 586 | 197 | 196 | 1400 | 5320 | 12 | 4376 | 5080 | 2240 | 320 | 29 | 0 | 0 | 0 | 0 | pass | pass | FAIL | pass | pass | pass | pass | pass | pass | FAIL |
| MAP24 | 1174 | 1109 | 1701 | 181 | 364 | 1954 | 697 | 696 | 2016 | 8685 | 16 | 6016 | 5328 | 1984 | 512 | 31 | 0 | 0 | 0 | 0 | pass | pass | FAIL | pass | pass | pass | pass | pass | pass | FAIL |
| MAP25 | 697 | 762 | 1079 | 147 | 276 | 1164 | 407 | 406 | 1881 | 7123 | 22 | 4120 | 7200 | 1088 | 368 | 37 | 0 | 0 | 0 | 0 | pass | pass | FAIL | pass | pass | pass | pass | pass | pass | FAIL |
| MAP26 | 818 | 850 | 1184 | 126 | 296 | 1305 | 453 | 452 | 900 | 4239 | 18 | 4584 | 3168 | 1120 | 312 | 38 | 0 | 2 | n/t | 0 | pass | pass | pass | pass | pass | pass | pass | pass | FAIL | FAIL |
| MAP27 | 864 | 946 | 1407 | 187 | 490 | 1522 | 484 | 483 | 1116 | 5290 | 21 | 3872 | 4480 | 1984 | 256 | 40 | 0 | 0 | 0 | 0 | pass | pass | FAIL | pass | pass | pass | pass | pass | pass | FAIL |
| MAP28 | 749 | 745 | 1057 | 116 | 266 | 1181 | 456 | 455 | 1400 | 5742 | 18 | 4456 | 5016 | 944 | 280 | 32 | 0 | 0 | 0 | 0 | pass | pass | pass | pass | pass | pass | pass | pass | pass | pass |
| MAP29 | 1180 | 1152 | 1653 | 205 | 276 | 1911 | 751 | 750 | 1406 | 6531 | 18 | 4816 | 4608 | 1576 | 968 | 38 | 0 | 0 | 0 | 0 | pass | pass | pass | pass | pass | pass | pass | FAIL | pass | FAIL |
| MAP30 | 82 | 93 | 131 | 22 | 95 | 137 | 43 | 42 | 484 | 1754 | 11 | 2696 | 2768 | 1792 | 640 | 17 | 0 | 2 | n/t | 0 | pass | pass | pass | pass | pass | pass | pass | FAIL | FAIL | FAIL |
| MAP31 | 635 | 686 | 783 | 80 | 317 | 814 | 229 | 228 | 3933 | 13435 | 14 | 8744 | 7168 | 1408 | 184 | 21 | 0 | 0 | 0 | 0 | pass | pass | FAIL | pass | FAIL | pass | pass | pass | pass | FAIL |
| MAP32 | 229 | 216 | 276 | 20 | 111 | 308 | 91 | 90 | 1824 | 6124 | 10 | 3968 | 7168 | 2688 | 256 | 21 | 0 | 0 | 0 | 0 | pass | pass | FAIL | pass | pass | pass | pass | pass | pass | FAIL |

maps 68
certificate (i) holds for 64 of 68
certificate (ii) holds for 46 of 68
certificate (iii) holds for 68 of 68
certificate (iv) holds for 67 of 68
certificate (v) holds for 68 of 68
certificate (vi) holds for 68 of 68
certificate (vii) holds for 58 of 68
certificate (i)–(vii) all hold for 39 of 68
C1 holds for 65 of 68
C2 holds for 56 of 68
C3 holds (where tested) for 55 of 68
closed-cell BLOCKMAP rule holds for 67 of 68
BLOCKMAP covers every vertex for 68 of 68
REJECT length sufficient for 68 of 68
2x2 window ≤ 128 lines for 68 of 68; maximum 101
geometry admitted under every rule for 33 of 68
maxima: linedefs 1764, sectors 348, cells 3933, words 13435, L1 5928, one-sided walls 1276
