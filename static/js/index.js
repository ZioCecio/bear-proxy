let savedServices = [];

function escapeHTML(text) {
    const p = document.createElement('p');
    p.appendChild(document.createTextNode(text));
    return p.innerHTML;
}

function base64ToArrayBuffer(base64) {
    var binaryString = atob(base64);
    var bytes = new Uint8Array(binaryString.length);
    for (var i = 0; i < binaryString.length; i++) {
        bytes[i] = binaryString.charCodeAt(i);
    }
    return bytes;
}

function parseRule(rule) {
    const buffer = base64ToArrayBuffer(rule.b64_rule);
    let formattedRule = '';
    for(const byte of buffer) {
        if(byte >= 32 && byte <= 126) {
            formattedRule += String.fromCharCode(byte);   
        } else {
            formattedRule += `\\x${byte.toString(16).padStart(2, '0')}`;
        }
    }

    rule.b64_rule = formattedRule;
}

async function getServices() {
    const servicesWrapper = await fetch('/services');
    const services = await servicesWrapper.json();
    savedServices = services;

    return services;
}

async function writeServices() {
    const services = await getServices();

    for(let service of services) {
        const option = `<option value='${service}'>${service}</option>`;
        $('#select-service').append(option);
    }
}

async function getRulesByServiceName(serviceName) {
    const rulesWrapper = await fetch(`/rules/filter/${serviceName}`);
    const rules = await rulesWrapper.json();

    for(const rule of rules) {
        parseRule(rule);
    }

    return rules;
}

function getRuleHtml(rule) {
    return `
        <li id="rule-${rule.id}" class="list-group-item">
            <div class="row">
                <div class="col-1 d-flex align-items-center justify-content-center">
                    ${rule.id}
                </div>
                <div class="col-10 d-flex align-items-center">
                    ${escapeHTML(rule.b64_rule)}
                </div>
                <div class="col-1 d-flex align-items-center justify-content-center">
                    <button class="btn btn-outline-danger" onclick="deleteRule(${rule.id})">
                        <i class="fa-regular fa-trash-can"></i>
                    </button>
                </div>
            </div>
        </li>
    `;
}

async function writeServicesAndRules() {
    for(const service of savedServices) {
        const rules = await getRulesByServiceName(service);

        const serviceHtml = `
            <div class="col-6">
                <div class="card">
                    <h5 class="card-header">${service}</h5>
                    <div class="card-body">
                        <ul id="${service}-rules-list" class="list-group"></ul>
                    </div>
                </div>
            </div>
        `;
        $(`#rules-div > div`).append(serviceHtml);

        for(const rule of rules) {
            const ruleHtml = getRuleHtml(rule);
            $(`#${service}-rules-list`).append(ruleHtml);
        }
    }
}

async function addRule() {
    const service = $('#select-service').val();
    const ruleText = $('#rule-text').val();
    const ruleType = $('input[name="rule-type-radio"]:checked').val();

    let error = false;
    if(service === 'default') {
        $('#select-service').addClass('is-invalid');
        error = true;
    }
    if(ruleText === '') {
        $('#rule-text').addClass('is-invalid');
        error = true;
    }

    if(error) {
        return;
    }

    const body = {
        service_name: service,
        rule_text: ruleText,
        rule_type: ruleType,
    }

    try {
        const response = await fetch('/rules', {
            method: 'POST',
            body: JSON.stringify(body),
            headers: {
                'Content-Type': 'application/json',
            }
        });

        const responseBody = await response.json();
        if(response.status === 400) {
            $('#rule-text-invalid-feedback').html(responseBody.message);
            $('#rule-text').addClass('is-invalid');

            return;
        }

        parseRule(responseBody);
        const ruleHtml = getRuleHtml(responseBody);
        $(`#${service}-rules-list`).append(ruleHtml);

        $('#add-success-toast').toast('show');
    } catch(error) {
        $('#error-toast').toast('show');
        console.log(error);
    }
    console.log(service, ruleText, ruleType);
}

async function deleteRule(ruleId) {
    try {
        const response = await fetch(`/rules/${ruleId}`, {
            method: 'DELETE',
        });

        if(response.status !== 200) {
            $('#error-toast').toast('show');
            return;
        }
        $('#delete-success-toast').toast('show');
        $(`#rule-${ruleId}`).remove();

    } catch(error) {
        console.log(error);
    }
}

function clearInvalid(id) {
    $(`#${id}`).removeClass('is-invalid');
    $('#rule-text-invalid-feedback').html('');
}

async function main() {
    await writeServices();
    await writeServicesAndRules();
}

$(document).onload(() => {
    main();
});