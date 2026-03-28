[Console]::OutputEncoding = [System.Text.Encoding]::UTF8

foreach ($arg in $args) {
  [Console]::Out.WriteLine("RONG_ARG:$arg")
}
