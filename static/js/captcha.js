// Taken from
// https://github.com/dchest/captcha/blob/master/capexample/main.go

function setSrcQuery(e, q) {
	var src  = e.src;
	var p = src.indexOf('?');
	if (p >= 0) {
		src = src.substr(0, p);
	}
	e.src = src + "?" + q
}

function reloadCaptcha() {
	setSrcQuery(document.getElementById('captcha'),
              "reload=" + (new Date()).getTime());
	return false;
}
