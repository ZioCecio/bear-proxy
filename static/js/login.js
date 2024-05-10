$(document).keypress((event) => {
    if(event.key == 'Enter') {
        tryLogin();
    }
});

async function tryLogin() {
    const password = $('#password-field').val();
    
    try {
        const response = await fetch('/get_token', {
            method: 'POST',
            body: JSON.stringify({ password }),
            headers: {
                'Content-Type': 'application/json',
            }
        });

        if(response.status !== 200) {
            $('#password-field').addClass('is-invalid');
            return;
        }

        location.reload();
    } catch(error) {
        console.log(error);
    }
}