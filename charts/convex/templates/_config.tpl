{{/* Derive application settings before delegating resource generation to common. */}}
{{- define "convex.config" -}}
{{- $v := .Values -}}
{{- $c := $v.convex -}}
{{- $container := $v.controllers.main.containers.main -}}
{{- if $v.controllers.main.enabled -}}
  {{- if not $c.publicUrl -}}
    {{- fail "convex.publicUrl is required (the exact browser origin, e.g. https://convex.example.com)" -}}
  {{- end -}}

  {{/* Normalize both common-library environment syntaxes before merging. */}}
  {{- $nativeEnv := $container.env -}}
  {{- $env := dict -}}
  {{- if kindIs "slice" $nativeEnv -}}
    {{- range $nativeEnv -}}
      {{- $_ := set $env .name (omit . "name") -}}
    {{- end -}}
  {{- else -}}
    {{- $env = deepCopy (default dict $nativeEnv) -}}
  {{- end -}}
  {{- $defaults := deepCopy $c.config -}}
  {{- range $name, $value := $env -}}
    {{- $_ := set $defaults $name $value -}}
  {{- end -}}
  {{- $env = $defaults -}}
  {{- $_ := set $env "CONVEX_PUBLIC_URL" $c.publicUrl -}}
  {{- $_ := set $env "CONVEX_BIND_ADDRESS" (printf "0.0.0.0:%v" $c.port) -}}
  {{- $_ := set $container "env" $env -}}

  {{- $envFrom := default list $container.envFrom -}}
  {{- with $c.existingSecret -}}
    {{- $envFrom = append $envFrom (dict "secretRef" (dict "name" .)) -}}
  {{- end -}}
  {{- $_ := set $container "envFrom" $envFrom -}}
  {{- if and (not $c.existingSecret) (not (get $env "DATABASE_URL")) (not $envFrom) -}}
    {{- fail "Provide DATABASE_URL via convex.existingSecret, convex.config, or controllers.main.containers.main.env/envFrom" -}}
  {{- end -}}

  {{/* Keep native list order, including Kubernetes $(VAR) expansion semantics. */}}
  {{- if kindIs "slice" $nativeEnv -}}
    {{- $native := list -}}
    {{- range $nativeEnv -}}
      {{- if not (has .name (list "CONVEX_PUBLIC_URL" "CONVEX_BIND_ADDRESS")) -}}
        {{- $_ := unset $defaults .name -}}
        {{- $native = append $native . -}}
      {{- end -}}
    {{- end -}}
    {{- $generated := list -}}
    {{- range $name, $value := $defaults -}}
      {{- $item := dict "name" $name -}}
      {{- if kindIs "map" $value -}}
        {{- $item = merge $item $value -}}
      {{- else -}}
        {{- $_ := set $item "value" $value -}}
      {{- end -}}
      {{- $generated = append $generated $item -}}
    {{- end -}}
    {{- $_ := set $container "env" (concat $generated $native) -}}
  {{- end -}}

  {{/* Preserve additional ports while keeping the HTTP listener in sync. */}}
  {{- $ports := list (dict "name" "http" "containerPort" ($c.port | int) "protocol" "TCP") -}}
  {{- range $container.ports -}}
    {{- if ne .name "http" -}}{{- $ports = append $ports . -}}{{- end -}}
  {{- end -}}
  {{- $_ := set $container "ports" $ports -}}
{{- end -}}
{{- end -}}
