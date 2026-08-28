# PVE RouterOS L6

---

## 1. SOFTWARE ID / License / HEX

---

- **SOFTWARE ID: TI09-7WK3 (L6)**

  **LICENSE KEY**:
  ```
  -----BEGIN MIKROTIK SOFTWARE KEY------------
  ...
  -----END MIKROTIK SOFTWARE KEY--------------
  ```
  **MBR HEX**:
  ```
  00000000000000000000BDE800000000E67A8F47AE86672FAE6D91DF19221453B34FE40E23F19E917107C449DDCB1D2061521816AD7730671B4CB226F1B0DB7448923C6297C49BDB3CCBF40AECBBCF0B
  ```

---

- **SOFTWARE ID: 4MZF-SFTR (L6)**

  **LICENSE KEY**:
  ```
  -----BEGIN MIKROTIK SOFTWARE KEY------------
  ...
  -----END MIKROTIK SOFTWARE KEY--------------
  ```
  **MBR HEX**:
  ```
  00000000000000000000BDE800000000080342D34683448A1C8E3952E5A5D315F1C5FB4E4EB419C94FB88170DF0290EE3F4DFB796ECA3034D93E934B3FC27169D6506C88F23FE508B26F83546C335A05
  ```

---

- **SOFTWARE ID: HHJH-UFWL (L6)**

  **LICENSE KEY**:
  ```
  -----BEGIN MIKROTIK SOFTWARE KEY------------
  ...
  -----END MIKROTIK SOFTWARE KEY--------------
  ```
  **MBR HEX**:
  ```
  00000000000000000000BDE800000000B08F6DA0CE6D8A13357403F0146B1DD227C5DEBFBD1B8260BE38DB0016D8B0BD110B34457997C8AC956FB7551081C1CB8DA79C0E6160A8DFE79F6FC38E543905
  ```

---

- **SOFTWARE ID: C7CU-PGT9 (L6)**

  **LICENSE KEY**:
  ```
  -----BEGIN MIKROTIK SOFTWARE KEY------------
  ...
  -----END MIKROTIK SOFTWARE KEY--------------
  ```
  **MBR HEX**:
  ```
  00000000000000000000BDE800000000F4E11772DEEAED8AF43668DA5EBDAD0846B694FFE9E77EFAE77E11A6049E4303B0B09DCEF8D9A647D643D1BAD4AF13B9659CCB11A06D3A9080096634E4E88B07
  ```

---

## 2. Collision Database

Pick the disk size you want and note down the **Model**, **Serial**, and **SOFTWARE ID**.

To search for a new disk size:

```bash
ros-serialgen search -s <GB> -t <threads> -c 0 -k keys.toml
```

**Structure**: rows are grouped **Level -> Software ID -> Size** (merged cells), with exactly **2 reserved row-slots** per (Software ID, Size) pair -- filled slots show a real collision entry, empty slots show `—`. **Identity** (`0x100-0x109`, 10 bytes) and **Marker** (`0x10A-0x10B`) are this project's standard all-zero-identity / `BDE8`-marker collision-search convention -- every entry below uses it.

<table>
<thead><tr><th>Level</th><th>Software ID</th><th>Size</th><th>Bytes</th><th>Model</th><th>Serial</th><th>Identity</th><th>Marker</th><th>Note</th></tr></thead>
<tbody>
<tr><td rowspan="152"><b>L6</b></td><td rowspan="38"><code>4MZF-SFTR</code></td><td rowspan="2">6G</td><td rowspan="2">6,442,450,944</td><td><code>ROS6G</code></td><td><code>00000005754879821902</code></td><td><code>00000000000000000000</code></td><td><code>BDE8</code></td><td></td></tr>
<tr><td>&mdash;</td><td>&mdash;</td><td>&mdash;</td><td>&mdash;</td><td>&mdash;</td></tr>
<tr><td rowspan="2">8G</td><td rowspan="2">7,918,460,928</td><td><code>SSD08G</code></td><td><code>HKHYPO14032703B0778</code></td><td><code>00000000000000000000</code></td><td><code>BDE8</code></td><td></td></tr>
<tr><td>&mdash;</td><td>&mdash;</td><td>&mdash;</td><td>&mdash;</td><td>&mdash;</td></tr>
<tr><td rowspan="2">8G</td><td rowspan="2">8,589,934,592</td><td><code>ROS8G</code></td><td><code>00000000106987476296</code></td><td><code>00000000000000000000</code></td><td><code>BDE8</code></td><td></td></tr>
<tr><td><code>ROS8G</code></td><td><code>00000000569989498985</code></td><td><code>00000000000000000000</code></td><td><code>BDE8</code></td><td></td></tr>
<tr><td rowspan="2">16G</td><td rowspan="2">17,179,869,184</td><td><code>ROS16G</code></td><td><code>00000000202155543391</code></td><td><code>00000000000000000000</code></td><td><code>BDE8</code></td><td></td></tr>
<tr><td>&mdash;</td><td>&mdash;</td><td>&mdash;</td><td>&mdash;</td><td>&mdash;</td></tr>
<tr><td rowspan="2">24G</td><td rowspan="2">25,769,803,776</td><td><code>cheerlon</code></td><td><code>00000000090681934458</code></td><td><code>00000000000000000000</code></td><td><code>BDE8</code></td><td></td></tr>
<tr><td>&mdash;</td><td>&mdash;</td><td>&mdash;</td><td>&mdash;</td><td>&mdash;</td></tr>
<tr><td rowspan="2">32G</td><td rowspan="2">31,675,383,808</td><td>&mdash;</td><td>&mdash;</td><td>&mdash;</td><td>&mdash;</td><td>&mdash;</td></tr>
<tr><td>&mdash;</td><td>&mdash;</td><td>&mdash;</td><td>&mdash;</td><td>&mdash;</td></tr>
<tr><td rowspan="2">32G</td><td rowspan="2">34,359,738,368</td><td><code>ROS32G</code></td><td><code>00000000233703762618</code></td><td><code>00000000000000000000</code></td><td><code>BDE8</code></td><td></td></tr>
<tr><td><code>ROS32G</code></td><td><code>00000000979618174623</code></td><td><code>00000000000000000000</code></td><td><code>BDE8</code></td><td></td></tr>
<tr><td rowspan="2">42G</td><td rowspan="2">45,097,156,608</td><td><code>ROS42G</code></td><td><code>00000001855210443015</code></td><td><code>00000000000000000000</code></td><td><code>BDE8</code></td><td></td></tr>
<tr><td>&mdash;</td><td>&mdash;</td><td>&mdash;</td><td>&mdash;</td><td>&mdash;</td></tr>
<tr><td rowspan="2">48G</td><td rowspan="2">51,539,607,552</td><td><code>ROS48G</code></td><td><code>00000000580121167237</code></td><td><code>00000000000000000000</code></td><td><code>BDE8</code></td><td></td></tr>
<tr><td>&mdash;</td><td>&mdash;</td><td>&mdash;</td><td>&mdash;</td><td>&mdash;</td></tr>
<tr><td rowspan="2">64G</td><td rowspan="2">64,023,257,088</td><td>&mdash;</td><td>&mdash;</td><td>&mdash;</td><td>&mdash;</td><td>&mdash;</td></tr>
<tr><td>&mdash;</td><td>&mdash;</td><td>&mdash;</td><td>&mdash;</td><td>&mdash;</td></tr>
<tr><td rowspan="2">64G</td><td rowspan="2">68,719,476,736</td><td><code>ROS64G</code></td><td><code>00000000585368148602</code></td><td><code>00000000000000000000</code></td><td><code>BDE8</code></td><td></td></tr>
<tr><td><code>ROS64G</code></td><td><code>00000001004796917412</code></td><td><code>00000000000000000000</code></td><td><code>BDE8</code></td><td></td></tr>
<tr><td rowspan="2">100G</td><td rowspan="2">107,374,182,400</td><td><code>ROS100G</code></td><td><code>00000000418756277141</code></td><td><code>00000000000000000000</code></td><td><code>BDE8</code></td><td></td></tr>
<tr><td><code>ROS100G</code></td><td><code>00000001311655212178</code></td><td><code>00000000000000000000</code></td><td><code>BDE8</code></td><td></td></tr>
<tr><td rowspan="2">118G</td><td rowspan="2">126,701,535,232</td><td><code>ROS118G</code></td><td><code>00000000498227777823</code></td><td><code>00000000000000000000</code></td><td><code>BDE8</code></td><td></td></tr>
<tr><td><code>ROS118G</code></td><td><code>00000000972873349134</code></td><td><code>00000000000000000000</code></td><td><code>BDE8</code></td><td></td></tr>
<tr><td rowspan="2">120G</td><td rowspan="2">128,849,018,880</td><td><code>ROS120G</code></td><td><code>00000000873014723475</code></td><td><code>00000000000000000000</code></td><td><code>BDE8</code></td><td></td></tr>
<tr><td><code>ROS120G</code></td><td><code>00000001517720626945</code></td><td><code>00000000000000000000</code></td><td><code>BDE8</code></td><td></td></tr>
<tr><td rowspan="2">128G</td><td rowspan="2">137,438,953,472</td><td><code>ROS128G</code></td><td><code>00000000311309782924</code></td><td><code>00000000000000000000</code></td><td><code>BDE8</code></td><td></td></tr>
<tr><td><code>ROS128G</code></td><td><code>00000000711951613799</code></td><td><code>00000000000000000000</code></td><td><code>BDE8</code></td><td></td></tr>
<tr><td rowspan="2">250G</td><td rowspan="2">268,435,456,000</td><td><code>ROS250G</code></td><td><code>00000000146612334244</code></td><td><code>00000000000000000000</code></td><td><code>BDE8</code></td><td></td></tr>
<tr><td><code>ROS250G</code></td><td><code>00000000403199458963</code></td><td><code>00000000000000000000</code></td><td><code>BDE8</code></td><td></td></tr>
<tr><td rowspan="2">256G</td><td rowspan="2">274,877,906,944</td><td><code>ROS256G</code></td><td><code>00000000031811615027</code></td><td><code>00000000000000000000</code></td><td><code>BDE8</code></td><td></td></tr>
<tr><td><code>ROS256G</code></td><td><code>00000001207922105396</code></td><td><code>00000000000000000000</code></td><td><code>BDE8</code></td><td></td></tr>
<tr><td rowspan="2">500G</td><td rowspan="2">536,870,912,000</td><td>&mdash;</td><td>&mdash;</td><td>&mdash;</td><td>&mdash;</td><td>&mdash;</td></tr>
<tr><td>&mdash;</td><td>&mdash;</td><td>&mdash;</td><td>&mdash;</td><td>&mdash;</td></tr>
<tr><td rowspan="2">512G</td><td rowspan="2">549,755,813,888</td><td>&mdash;</td><td>&mdash;</td><td>&mdash;</td><td>&mdash;</td><td>&mdash;</td></tr>
<tr><td>&mdash;</td><td>&mdash;</td><td>&mdash;</td><td>&mdash;</td><td>&mdash;</td></tr>
<tr><td rowspan="38"><code>C7CU-PGT9</code></td><td rowspan="2">6G</td><td rowspan="2">6,442,450,944</td><td><code>ROS6G</code></td><td><code>00000000401012206606</code></td><td><code>00000000000000000000</code></td><td><code>BDE8</code></td><td></td></tr>
<tr><td><code>ROS6G</code></td><td><code>00000001731995041625</code></td><td><code>00000000000000000000</code></td><td><code>BDE8</code></td><td></td></tr>
<tr><td rowspan="2">8G</td><td rowspan="2">7,918,460,928</td><td>&mdash;</td><td>&mdash;</td><td>&mdash;</td><td>&mdash;</td><td>&mdash;</td></tr>
<tr><td>&mdash;</td><td>&mdash;</td><td>&mdash;</td><td>&mdash;</td><td>&mdash;</td></tr>
<tr><td rowspan="2">8G</td><td rowspan="2">8,589,934,592</td><td><code>ROS8G</code></td><td><code>00000000329230626319</code></td><td><code>00000000000000000000</code></td><td><code>BDE8</code></td><td></td></tr>
<tr><td><code>ROS8G</code></td><td><code>00000001979270362453</code></td><td><code>00000000000000000000</code></td><td><code>BDE8</code></td><td></td></tr>
<tr><td rowspan="2">16G</td><td rowspan="2">17,179,869,184</td><td><code>ROS16G</code></td><td><code>00000000920288220129</code></td><td><code>00000000000000000000</code></td><td><code>BDE8</code></td><td></td></tr>
<tr><td>&mdash;</td><td>&mdash;</td><td>&mdash;</td><td>&mdash;</td><td>&mdash;</td></tr>
<tr><td rowspan="2">24G</td><td rowspan="2">25,769,803,776</td><td><code>cheerlon</code></td><td><code>00000001978887615673</code></td><td><code>00000000000000000000</code></td><td><code>BDE8</code></td><td></td></tr>
<tr><td><code>cheerlon</code></td><td><code>00000002382131429301</code></td><td><code>00000000000000000000</code></td><td><code>BDE8</code></td><td></td></tr>
<tr><td rowspan="2">32G</td><td rowspan="2">31,675,383,808</td><td>&mdash;</td><td>&mdash;</td><td>&mdash;</td><td>&mdash;</td><td>&mdash;</td></tr>
<tr><td>&mdash;</td><td>&mdash;</td><td>&mdash;</td><td>&mdash;</td><td>&mdash;</td></tr>
<tr><td rowspan="2">32G</td><td rowspan="2">34,359,738,368</td><td><code>ROS32G</code></td><td><code>00000003204113283903</code></td><td><code>00000000000000000000</code></td><td><code>BDE8</code></td><td></td></tr>
<tr><td><code>ROS32G</code></td><td><code>00000003250933554198</code></td><td><code>00000000000000000000</code></td><td><code>BDE8</code></td><td></td></tr>
<tr><td rowspan="2">42G</td><td rowspan="2">45,097,156,608</td><td><code>ROS42G</code></td><td><code>00000002074332007468</code></td><td><code>00000000000000000000</code></td><td><code>BDE8</code></td><td></td></tr>
<tr><td><code>ROS42G</code></td><td><code>00000002408814666185</code></td><td><code>00000000000000000000</code></td><td><code>BDE8</code></td><td></td></tr>
<tr><td rowspan="2">48G</td><td rowspan="2">51,539,607,552</td><td><code>ROS48G</code></td><td><code>00000000621828037033</code></td><td><code>00000000000000000000</code></td><td><code>BDE8</code></td><td></td></tr>
<tr><td>&mdash;</td><td>&mdash;</td><td>&mdash;</td><td>&mdash;</td><td>&mdash;</td></tr>
<tr><td rowspan="2">64G</td><td rowspan="2">64,023,257,088</td><td><code>SSD64G2016</code></td><td><code>HYSSD-20160419B79028</code></td><td><code>00000000000000000000</code></td><td><code>BDE8</code></td><td></td></tr>
<tr><td>&mdash;</td><td>&mdash;</td><td>&mdash;</td><td>&mdash;</td><td>&mdash;</td></tr>
<tr><td rowspan="2">64G</td><td rowspan="2">68,719,476,736</td><td><code>ROS64G</code></td><td><code>00000000350481748276</code></td><td><code>00000000000000000000</code></td><td><code>BDE8</code></td><td></td></tr>
<tr><td><code>ROS64G</code></td><td><code>00000001250109115573</code></td><td><code>00000000000000000000</code></td><td><code>BDE8</code></td><td></td></tr>
<tr><td rowspan="2">100G</td><td rowspan="2">107,374,182,400</td><td><code>ROS100G</code></td><td><code>00000001467083308066</code></td><td><code>00000000000000000000</code></td><td><code>BDE8</code></td><td></td></tr>
<tr><td><code>ROS100G</code></td><td><code>00000001528416122910</code></td><td><code>00000000000000000000</code></td><td><code>BDE8</code></td><td></td></tr>
<tr><td rowspan="2">118G</td><td rowspan="2">126,701,535,232</td><td><code>ROS118G</code></td><td><code>00000001871808755962</code></td><td><code>00000000000000000000</code></td><td><code>BDE8</code></td><td></td></tr>
<tr><td><code>ROS118G</code></td><td><code>00000007334804203579</code></td><td><code>00000000000000000000</code></td><td><code>BDE8</code></td><td></td></tr>
<tr><td rowspan="2">120G</td><td rowspan="2">128,849,018,880</td><td><code>ROS120G</code></td><td><code>00000000669517804839</code></td><td><code>00000000000000000000</code></td><td><code>BDE8</code></td><td></td></tr>
<tr><td><code>ROS120G</code></td><td><code>00000001124777877709</code></td><td><code>00000000000000000000</code></td><td><code>BDE8</code></td><td></td></tr>
<tr><td rowspan="2">128G</td><td rowspan="2">137,438,953,472</td><td><code>ROS128G</code></td><td><code>00000000373541048649</code></td><td><code>00000000000000000000</code></td><td><code>BDE8</code></td><td></td></tr>
<tr><td><code>ROS128G</code></td><td><code>00000001022238657812</code></td><td><code>00000000000000000000</code></td><td><code>BDE8</code></td><td></td></tr>
<tr><td rowspan="2">250G</td><td rowspan="2">268,435,456,000</td><td><code>ROS250G</code></td><td><code>00000000497544036420</code></td><td><code>00000000000000000000</code></td><td><code>BDE8</code></td><td></td></tr>
<tr><td><code>ROS250G</code></td><td><code>00000001097783036676</code></td><td><code>00000000000000000000</code></td><td><code>BDE8</code></td><td></td></tr>
<tr><td rowspan="2">256G</td><td rowspan="2">274,877,906,944</td><td><code>ROS256G</code></td><td><code>00000001185984725662</code></td><td><code>00000000000000000000</code></td><td><code>BDE8</code></td><td></td></tr>
<tr><td><code>ROS256G</code></td><td><code>00000001661347186078</code></td><td><code>00000000000000000000</code></td><td><code>BDE8</code></td><td></td></tr>
<tr><td rowspan="2">500G</td><td rowspan="2">536,870,912,000</td><td>&mdash;</td><td>&mdash;</td><td>&mdash;</td><td>&mdash;</td><td>&mdash;</td></tr>
<tr><td>&mdash;</td><td>&mdash;</td><td>&mdash;</td><td>&mdash;</td><td>&mdash;</td></tr>
<tr><td rowspan="2">512G</td><td rowspan="2">549,755,813,888</td><td>&mdash;</td><td>&mdash;</td><td>&mdash;</td><td>&mdash;</td><td>&mdash;</td></tr>
<tr><td>&mdash;</td><td>&mdash;</td><td>&mdash;</td><td>&mdash;</td><td>&mdash;</td></tr>
<tr><td rowspan="38"><code>HHJH-UFWL</code></td><td rowspan="2">6G</td><td rowspan="2">6,442,450,944</td><td><code>ROS6G</code></td><td><code>00000000931789296514</code></td><td><code>00000000000000000000</code></td><td><code>BDE8</code></td><td></td></tr>
<tr><td><code>ROS6G</code></td><td><code>00000003381278892880</code></td><td><code>00000000000000000000</code></td><td><code>BDE8</code></td><td></td></tr>
<tr><td rowspan="2">8G</td><td rowspan="2">7,918,460,928</td><td>&mdash;</td><td>&mdash;</td><td>&mdash;</td><td>&mdash;</td><td>&mdash;</td></tr>
<tr><td>&mdash;</td><td>&mdash;</td><td>&mdash;</td><td>&mdash;</td><td>&mdash;</td></tr>
<tr><td rowspan="2">8G</td><td rowspan="2">8,589,934,592</td><td><code>ROS8G</code></td><td><code>00000002124408570543</code></td><td><code>00000000000000000000</code></td><td><code>BDE8</code></td><td></td></tr>
<tr><td><code>ROS8G</code></td><td><code>00000002551851332131</code></td><td><code>00000000000000000000</code></td><td><code>BDE8</code></td><td></td></tr>
<tr><td rowspan="2">16G</td><td rowspan="2">17,179,869,184</td><td><code>ROS16G</code></td><td><code>00000000424056873476</code></td><td><code>00000000000000000000</code></td><td><code>BDE8</code></td><td></td></tr>
<tr><td>&mdash;</td><td>&mdash;</td><td>&mdash;</td><td>&mdash;</td><td>&mdash;</td></tr>
<tr><td rowspan="2">24G</td><td rowspan="2">25,769,803,776</td><td><code>cheerlon</code></td><td><code>00000002242405383283</code></td><td><code>00000000000000000000</code></td><td><code>BDE8</code></td><td></td></tr>
<tr><td><code>cheerlon</code></td><td><code>00000002319333948983</code></td><td><code>00000000000000000000</code></td><td><code>BDE8</code></td><td></td></tr>
<tr><td rowspan="2">32G</td><td rowspan="2">31,675,383,808</td><td><code>SSD32G</code></td><td><code>SZHYPO14090903D0164</code></td><td><code>00000000000000000000</code></td><td><code>BDE8</code></td><td></td></tr>
<tr><td>&mdash;</td><td>&mdash;</td><td>&mdash;</td><td>&mdash;</td><td>&mdash;</td></tr>
<tr><td rowspan="2">32G</td><td rowspan="2">34,359,738,368</td><td><code>ROS32G</code></td><td><code>00000000850750679208</code></td><td><code>00000000000000000000</code></td><td><code>BDE8</code></td><td></td></tr>
<tr><td><code>ROS32G</code></td><td><code>00000003192153037237</code></td><td><code>00000000000000000000</code></td><td><code>BDE8</code></td><td></td></tr>
<tr><td rowspan="2">42G</td><td rowspan="2">45,097,156,608</td><td><code>ROS42G</code></td><td><code>00000002448898419424</code></td><td><code>00000000000000000000</code></td><td><code>BDE8</code></td><td></td></tr>
<tr><td>&mdash;</td><td>&mdash;</td><td>&mdash;</td><td>&mdash;</td><td>&mdash;</td></tr>
<tr><td rowspan="2">48G</td><td rowspan="2">51,539,607,552</td><td><code>ROS48G</code></td><td><code>00000000470627909740</code></td><td><code>00000000000000000000</code></td><td><code>BDE8</code></td><td></td></tr>
<tr><td>&mdash;</td><td>&mdash;</td><td>&mdash;</td><td>&mdash;</td><td>&mdash;</td></tr>
<tr><td rowspan="2">64G</td><td rowspan="2">64,023,257,088</td><td>&mdash;</td><td>&mdash;</td><td>&mdash;</td><td>&mdash;</td><td>&mdash;</td></tr>
<tr><td>&mdash;</td><td>&mdash;</td><td>&mdash;</td><td>&mdash;</td><td>&mdash;</td></tr>
<tr><td rowspan="2">64G</td><td rowspan="2">68,719,476,736</td><td><code>ROS64G</code></td><td><code>00000000508223321551</code></td><td><code>00000000000000000000</code></td><td><code>BDE8</code></td><td></td></tr>
<tr><td><code>ROS64G</code></td><td><code>00000001078437024732</code></td><td><code>00000000000000000000</code></td><td><code>BDE8</code></td><td></td></tr>
<tr><td rowspan="2">100G</td><td rowspan="2">107,374,182,400</td><td><code>ROS100G</code></td><td><code>00000000721135158938</code></td><td><code>00000000000000000000</code></td><td><code>BDE8</code></td><td></td></tr>
<tr><td><code>ROS100G</code></td><td><code>00000002027568720134</code></td><td><code>00000000000000000000</code></td><td><code>BDE8</code></td><td></td></tr>
<tr><td rowspan="2">118G</td><td rowspan="2">126,701,535,232</td><td><code>ROS118G</code></td><td><code>00000002160591072734</code></td><td><code>00000000000000000000</code></td><td><code>BDE8</code></td><td></td></tr>
<tr><td><code>ROS118G</code></td><td><code>00000002801992148223</code></td><td><code>00000000000000000000</code></td><td><code>BDE8</code></td><td></td></tr>
<tr><td rowspan="2">120G</td><td rowspan="2">128,849,018,880</td><td><code>ROS120G</code></td><td><code>00000000219025879553</code></td><td><code>00000000000000000000</code></td><td><code>BDE8</code></td><td></td></tr>
<tr><td><code>ROS120G</code></td><td><code>00000000906225324758</code></td><td><code>00000000000000000000</code></td><td><code>BDE8</code></td><td></td></tr>
<tr><td rowspan="2">128G</td><td rowspan="2">137,438,953,472</td><td><code>ROS128G</code></td><td><code>00000000837794059054</code></td><td><code>00000000000000000000</code></td><td><code>BDE8</code></td><td></td></tr>
<tr><td><code>ROS128G</code></td><td><code>00000001689719628818</code></td><td><code>00000000000000000000</code></td><td><code>BDE8</code></td><td></td></tr>
<tr><td rowspan="2">250G</td><td rowspan="2">268,435,456,000</td><td><code>ROS250G</code></td><td><code>00000002743409389567</code></td><td><code>00000000000000000000</code></td><td><code>BDE8</code></td><td></td></tr>
<tr><td>&mdash;</td><td>&mdash;</td><td>&mdash;</td><td>&mdash;</td><td>&mdash;</td></tr>
<tr><td rowspan="2">256G</td><td rowspan="2">274,877,906,944</td><td><code>ROS256G</code></td><td><code>00000001433715304507</code></td><td><code>00000000000000000000</code></td><td><code>BDE8</code></td><td></td></tr>
<tr><td>&mdash;</td><td>&mdash;</td><td>&mdash;</td><td>&mdash;</td><td>&mdash;</td></tr>
<tr><td rowspan="2">500G</td><td rowspan="2">536,870,912,000</td><td>&mdash;</td><td>&mdash;</td><td>&mdash;</td><td>&mdash;</td><td>&mdash;</td></tr>
<tr><td>&mdash;</td><td>&mdash;</td><td>&mdash;</td><td>&mdash;</td><td>&mdash;</td></tr>
<tr><td rowspan="2">512G</td><td rowspan="2">549,755,813,888</td><td>&mdash;</td><td>&mdash;</td><td>&mdash;</td><td>&mdash;</td><td>&mdash;</td></tr>
<tr><td>&mdash;</td><td>&mdash;</td><td>&mdash;</td><td>&mdash;</td><td>&mdash;</td></tr>
<tr><td rowspan="38"><code>TI09-7WK3</code></td><td rowspan="2">6G</td><td rowspan="2">6,442,450,944</td><td><code>VMware Virtual IDE Hard Drive</code></td><td><code>00000000000000000001</code></td><td><code>00000000000000000000</code></td><td><code>BDE8</code></td><td></td></tr>
<tr><td><code>ROS6G</code></td><td><code>00000003796372007447</code></td><td><code>00000000000000000000</code></td><td><code>BDE8</code></td><td></td></tr>
<tr><td rowspan="2">8G</td><td rowspan="2">7,918,460,928</td><td>&mdash;</td><td>&mdash;</td><td>&mdash;</td><td>&mdash;</td><td>&mdash;</td></tr>
<tr><td>&mdash;</td><td>&mdash;</td><td>&mdash;</td><td>&mdash;</td><td>&mdash;</td></tr>
<tr><td rowspan="2">8G</td><td rowspan="2">8,589,934,592</td><td><code>ROS8G</code></td><td><code>00000000611884626689</code></td><td><code>00000000000000000000</code></td><td><code>BDE8</code></td><td></td></tr>
<tr><td><code>ROS8G</code></td><td><code>00000001191734117396</code></td><td><code>00000000000000000000</code></td><td><code>BDE8</code></td><td></td></tr>
<tr><td rowspan="2">16G</td><td rowspan="2">17,179,869,184</td><td><code>ROS16G</code></td><td><code>00000001386919268807</code></td><td><code>00000000000000000000</code></td><td><code>BDE8</code></td><td></td></tr>
<tr><td><code>ROS16G</code></td><td><code>00000001522667870999</code></td><td><code>00000000000000000000</code></td><td><code>BDE8</code></td><td></td></tr>
<tr><td rowspan="2">24G</td><td rowspan="2">25,769,803,776</td><td><code>cheerlon</code></td><td><code>00000000868749092790</code></td><td><code>00000000000000000000</code></td><td><code>BDE8</code></td><td></td></tr>
<tr><td><code>cheerlon</code></td><td><code>00000002241967910358</code></td><td><code>00000000000000000000</code></td><td><code>BDE8</code></td><td></td></tr>
<tr><td rowspan="2">32G</td><td rowspan="2">31,675,383,808</td><td>&mdash;</td><td>&mdash;</td><td>&mdash;</td><td>&mdash;</td><td>&mdash;</td></tr>
<tr><td>&mdash;</td><td>&mdash;</td><td>&mdash;</td><td>&mdash;</td><td>&mdash;</td></tr>
<tr><td rowspan="2">32G</td><td rowspan="2">34,359,738,368</td><td><code>ROS32G</code></td><td><code>00000000031682233604</code></td><td><code>00000000000000000000</code></td><td><code>BDE8</code></td><td></td></tr>
<tr><td><code>ROS32G</code></td><td><code>00000001334690141671</code></td><td><code>00000000000000000000</code></td><td><code>BDE8</code></td><td></td></tr>
<tr><td rowspan="2">42G</td><td rowspan="2">45,097,156,608</td><td><code>n4X7W6eSOxyxUhOd</code></td><td><code>G4HQT594JN8VLY0FGN9</code></td><td><code>00000000000000000000</code></td><td><code>BDE8</code></td><td></td></tr>
<tr><td><code>ROS42G</code></td><td><code>00000002201539438409</code></td><td><code>00000000000000000000</code></td><td><code>BDE8</code></td><td></td></tr>
<tr><td rowspan="2">48G</td><td rowspan="2">51,539,607,552</td><td><code>ROS48G</code></td><td><code>00000000398318370243</code></td><td><code>00000000000000000000</code></td><td><code>BDE8</code></td><td></td></tr>
<tr><td><code>ROS48G</code></td><td><code>00000000467597837523</code></td><td><code>00000000000000000000</code></td><td><code>BDE8</code></td><td></td></tr>
<tr><td rowspan="2">64G</td><td rowspan="2">64,023,257,088</td><td>&mdash;</td><td>&mdash;</td><td>&mdash;</td><td>&mdash;</td><td>&mdash;</td></tr>
<tr><td>&mdash;</td><td>&mdash;</td><td>&mdash;</td><td>&mdash;</td><td>&mdash;</td></tr>
<tr><td rowspan="2">64G</td><td rowspan="2">68,719,476,736</td><td><code>ROS64G</code></td><td><code>00000001685383959455</code></td><td><code>00000000000000000000</code></td><td><code>BDE8</code></td><td></td></tr>
<tr><td><code>ROS64G</code></td><td><code>00000002310691435906</code></td><td><code>00000000000000000000</code></td><td><code>BDE8</code></td><td></td></tr>
<tr><td rowspan="2">100G</td><td rowspan="2">107,374,182,400</td><td><code>ROS100G</code></td><td><code>00000002012064574584</code></td><td><code>00000000000000000000</code></td><td><code>BDE8</code></td><td></td></tr>
<tr><td><code>ROS100G</code></td><td><code>00000002496386055722</code></td><td><code>00000000000000000000</code></td><td><code>BDE8</code></td><td></td></tr>
<tr><td rowspan="2">118G</td><td rowspan="2">126,701,535,232</td><td><code>ROS118G</code></td><td><code>00000004401066210070</code></td><td><code>00000000000000000000</code></td><td><code>BDE8</code></td><td></td></tr>
<tr><td><code>ROS118G</code></td><td><code>00000005271967480605</code></td><td><code>00000000000000000000</code></td><td><code>BDE8</code></td><td></td></tr>
<tr><td rowspan="2">120G</td><td rowspan="2">128,849,018,880</td><td><code>ROS120G</code></td><td><code>00000000184423415344</code></td><td><code>00000000000000000000</code></td><td><code>BDE8</code></td><td></td></tr>
<tr><td><code>ROS120G</code></td><td><code>00000001180705176578</code></td><td><code>00000000000000000000</code></td><td><code>BDE8</code></td><td></td></tr>
<tr><td rowspan="2">128G</td><td rowspan="2">137,438,953,472</td><td><code>ROS128G</code></td><td><code>00000000319572260957</code></td><td><code>00000000000000000000</code></td><td><code>BDE8</code></td><td></td></tr>
<tr><td><code>ROS128G</code></td><td><code>00000000897921335928</code></td><td><code>00000000000000000000</code></td><td><code>BDE8</code></td><td></td></tr>
<tr><td rowspan="2">250G</td><td rowspan="2">268,435,456,000</td><td><code>ROS250G</code></td><td><code>00000001836339689557</code></td><td><code>00000000000000000000</code></td><td><code>BDE8</code></td><td></td></tr>
<tr><td>&mdash;</td><td>&mdash;</td><td>&mdash;</td><td>&mdash;</td><td>&mdash;</td></tr>
<tr><td rowspan="2">256G</td><td rowspan="2">274,877,906,944</td><td><code>ROS256G</code></td><td><code>00000002184192056376</code></td><td><code>00000000000000000000</code></td><td><code>BDE8</code></td><td></td></tr>
<tr><td><code>ROS256G</code></td><td><code>00000002225069582042</code></td><td><code>00000000000000000000</code></td><td><code>BDE8</code></td><td></td></tr>
<tr><td rowspan="2">500G</td><td rowspan="2">536,870,912,000</td><td><code>ROS500G</code></td><td><code>00000000082620520955</code></td><td><code>00000000000000000000</code></td><td><code>BDE8</code></td><td></td></tr>
<tr><td>&mdash;</td><td>&mdash;</td><td>&mdash;</td><td>&mdash;</td><td>&mdash;</td></tr>
<tr><td rowspan="2">512G</td><td rowspan="2">549,755,813,888</td><td><code>ROS512G</code></td><td><code>00000000037935077152</code></td><td><code>00000000000000000000</code></td><td><code>BDE8</code></td><td></td></tr>
<tr><td>&mdash;</td><td>&mdash;</td><td>&mdash;</td><td>&mdash;</td><td>&mdash;</td></tr>
</tbody>
</table>

---

## 3. Create VM

SSH into the PVE host. Using **16G** as an example (VM ID 100):

```bash
qm create 100 \
  --name RouterOS \
  --ostype l26 \
  --bios ovmf \
  --efidisk0 local-lvm:1,efitype=4m,format=raw \
  --cores 1 --memory 256 \
  --ide2 local:iso/mikrotik-7.23.2.iso,media=cdrom \
  --net0 virtio,bridge=vmbr0 \
  --boot "order=ide2" \
  --scsihw virtio-scsi-single

qm set 100 --delete scsi0
mkdir -p /var/lib/vz/images/100
qemu-img create -f qcow2 /var/lib/vz/images/100/vm-100-disk.qcow2 17179869184
qm set 100 --ide0 local:100/vm-100-disk.qcow2,model=ROS16G,serial=00000000202155543391
```

> For model names containing spaces (e.g. the 6G VMware scheme), URL-encode the space as `%20`:
> ```
> qm set 100 --ide0 local:100/vm-100-disk.qcow2,model=VMware%20Virtual%20IDE%20Hard%20Drive,serial=00000000000000000001
> ```

For other disk sizes: replace the byte count in `qemu-img create`, plus `serial=` and `model=` (look them up in the collision database above).

---

## 4. Install RouterOS

1. PVE Web UI → start the VM → Console
2. `a` select all packages → `i` install → `y` confirm formatting
3. Once installation completes → **Stop the VM**

---

## 5. Activation

Look up the **SOFTWARE ID** from the collision database, then find the matching **HEX** or **Key text** in Section 1.

### 5.1 MBR write method

```bash
modprobe nbd max_part=8
qemu-nbd --connect=/dev/nbd0 /var/lib/vz/images/100/vm-100-disk.qcow2
sleep 1

echo -n '00000000000000000000BDE800000000F4E11772DEEAED8AF43668DA5EBDAD0846B694FFE9E77EFAE77E11A6049E4303B0B09DCEF8D9A647D643D1BAD4AF13B9659CCB11A06D3A9080096634E4E88B07' | xxd -r -p | dd of=/dev/nbd0 bs=1 seek=256 count=80 conv=notrunc

hexdump -C -s 0x100 -n 80 /dev/nbd0
qemu-nbd --disconnect /dev/nbd0
```

> The example above writes the HEX for **4MZF-SFTR**. For a different SOFTWARE ID, replace the content of `echo -n '...'` (copy it from Section 1).

### 5.2 Terminal import method

Start the VM, then paste the following directly into the Console (using **4MZF-SFTR** as an example):

```
/system/license/import "-----BEGIN MIKROTIK SOFTWARE KEY------------
...
-----END MIKROTIK SOFTWARE KEY--------------"
```

When prompted `Reboot? [y/N]:` → enter `y`

> For a different SOFTWARE ID, replace the Key text inside the quotes (copy it from Section 1).

### 5.3 Key HTTP import method

No shutdown required — start the VM directly after installation, then download the Key file over HTTP.

**On the PVE host, prepare the Key file and an HTTP server:**

```bash
mkdir -p /tmp/serve
cat > /tmp/serve/license.key << 'EOF'
-----BEGIN MIKROTIK SOFTWARE KEY------------
...
-----END MIKROTIK SOFTWARE KEY--------------
EOF
cd /tmp/serve && python3 -m http.server 8080 &
ip addr add 10.255.255.1/24 dev vmbr0 2>/dev/null
```

> The example above uses the Key for **4MZF-SFTR**. For a different SOFTWARE ID, replace the Key text (copy it from Section 1).

**RouterOS Console:**

```
/ip address add address=10.255.255.2/24 interface=ether1
/tool fetch url="http://10.255.255.1:8080/license.key" dst-path=license.key
/system license import file-name=license.key
```

When prompted `Reboot? [y/N]:` → enter `y`

> ⚠️ The file must have a `.key` extension.

---

## 6. Boot Verification

For the MBR method, remove the CD-ROM drive first:

```bash
qm set 100 --delete ide2
qm set 100 --boot ''
qm start 100
```

For the Key import method, the VM boots into RouterOS automatically after reboot.

Verify in the Console:

```
/system license print
```

```
  software-id: 4MZF-SFTR
       nlevel: 6
     features:
```

`nlevel: 6` = success ✅

---

## FAQ

**Q: Why does the toolkit use the name `ros-serialgen`?**
A: This is the collision search tool used to generate the serial numbers listed in Section 2. Given a target disk size and a known SOFTWARE ID, it searches for a serial that produces a SOFTWARE ID collision.

**Q: What if my disk size isn't in the table?**
A: Run `ros-serialgen search -s <GB> -t <threads>` to find a new collision for your target size, then add the resulting entry to the collision database.

**Q: The VM boots but shows `nlevel: 0` or no license — what went wrong?**
A: Confirm you installed RouterOS first and wrote the MBR (or imported the key) afterward — the installer overwrites bytes 0x10A-0x10B, so writing the MBR before installation destroys the collision. Also confirm the serial and model used to create the disk exactly match the row you picked from the collision database.

**Q: Can I reuse a serial from the table for a completely different disk size?**
A: No. Each serial/model combination was collision-searched against `sector_val` for its listed byte count. Using it with a different sector count invalidates the SOFTWARE ID match.
