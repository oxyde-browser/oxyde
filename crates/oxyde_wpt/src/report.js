// Remplace `/resources/testharnessreport.js` : rend les résultats au lanceur
// par la console, une ligne par sous-test puis une ligne pour le fichier.
(function () {
  function clean(s) {
    return String(s).replace(/[\t\r\n]+/g, " ");
  }
  setup({ output: false });
  add_completion_callback(function (tests, status) {
    for (var i = 0; i < tests.length; i++) {
      var t = tests[i];
      console.log("WPT-T\t" + t.status + "\t" + clean(t.name) + "\t" + clean(t.message || ""));
    }
    console.log("WPT-H\t" + status.status + "\t" + clean(status.message || ""));
  });
})();
