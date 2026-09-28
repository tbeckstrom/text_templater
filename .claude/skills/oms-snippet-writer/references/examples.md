# Examples

Five requests with the expected response. Match their shape and density: the first three are op-note prose; the last two are clinic form sections built for speed (form-design.md), using USES, TEETH, `?` lines, PARTS, conditions and flags.

<EXAMPLES>

<EXAMPLE PROMPT>
Indications section for op note related to enucleation and curettage
</EXAMPLE PROMPT>

<EXAMPLE RESPONSE>
SNIPPET: op_indications_enucleation_cyst
PREFIX: enuc
TITLE: Indications - enucleation & curettage
TEXT:
Given the presence of a LESION_DESCRIPTION of the LATERALITY MAXILLA_MANDIBLE with potential for continued expansion and structural compromise of adjacent structures, definitive surgical management was recommended.

Enucleation with curettage was advised in order to remove the pathologic tissue, obtain definitive histopathologic diagnosis, and reduce the risk of lesion progression while minimizing surgical morbidity. OPTIONAL_ADDS

The risks, benefits, and alternatives to operative management, including observation, were discussed with the patient. After discussion, the decision was made to proceed with enucleation and curettage of the ***(lesion type) lesion with grafting of the residual defect.

LIST: LESION_DESCRIPTION (required)
. Radiolucent lesion
  - radiolucent lesion
. Biopsy proven
  - biopsy proven ***(diagnosis)

LIST: LATERALITY (required)
- right
- left
- bilateral

LIST: MAXILLA_MANDIBLE (required)
- maxilla
- mandible

MULTILIST(space): OPTIONAL_ADDS
. Extraction Single
  - Given the intimate association of the lesion with tooth ***(FDI tooth), extraction of this tooth was discussed as a potential measure to reduce risk of recurrence and facilitate complete lesion removal. The patient elected to PROCEED_DEFER the proposed extraction after discussion of the risks and benefits.
. Extraction Multiple
  - Given the intimate association of the lesion with teeth ***(FDI teeth), extraction of these teeth was discussed as a potential measure to reduce risk of recurrence and facilitate complete lesion removal. The patient elected to PROCEED_DEFER the proposed extractions after discussion of the risks and benefits.
. Possible RCT / Ext in future
  - The potential need for future endodontic therapy or extractions was discussed given the proximity of the lesion and anticipated instrumentation near the roots of adjacent teeth.
. Bone Grafting
  - Given the anticipated residual bony defect following removal of the lesion, grafting of the defect with BONE_GRAFT_TYPES was planned.
. Adjunctive 5-FU
  - Adjunctive local therapy with application of 5-fluorouracil to the surgical site to reduce recurrence risk was discussed as part of surgical management.

LIST: PROCEED_DEFER (required)
- proceed with
- defer

MULTILIST(and): BONE_GRAFT_TYPES
- particulate allograft
- particulate xenograft
- BMP
- autogenous bone graft
</EXAMPLE RESPONSE>

<EXAMPLE PROMPT>
procedure details snippet for submandibular approach to the mandible
</EXAMPLE PROMPT>

<EXAMPLE RESPONSE>
SNIPPET: common_local_anesthesia_admin
PREFIX: local
TITLE: Local anesthesia
TEXT:
LOCAL_TYPES was infiltrated subcutaneously along the planned incision.

MULTILIST(and): LOCAL_TYPES
- 1% lidocaine with 1:100,000 epinephrine (default)
- 0.25% bupivacaine with 1:200,000 epinephrine
- 0.5% bupivacaine with 1:200,000 epinephrine

=====

SNIPPET: op_approach_submandibular
PREFIX: submand
TITLE: Submandibular approach
TEXT:
Attention was directed to the submandibular region. A ***(incision length)cm skin incision was marked approximately 2 cm inferior to the inferior border of the mandible, oriented parallel to a natural skin crease. @common_local_anesthesia_admin@

The incision was carried through sharply skin and subcutaneous tissue to the level of the platysma. Subcutaneous undermining was performed superiorly and inferiorly to facilitate retraction and closure. The platysma muscle was divided in a controlled fashion, exposing the superficial layer of deep cervical fascia.

Blunt and sharp dissection through the superficial layer of deep cervical fascia was performed at the level of the skin incision, followed by superior dissection toward the mandible. The submandibular gland was encountered and retracted inferiorly as needed. FACIAL_VESSELS_STATEMENT MARGINAL_MANDIBULAR_STATEMENT

Dissection proceeded superiorly toward the inferior border of the mandible until the pterygomasseteric sling was identified. The sling was divided sharply to expose the inferior border and angle of the mandible.

LIST: FACIAL_VESSELS_STATEMENT
. Facial vessels not seen (default)
  - The facial vessels were not encountered within the operative field.
. Facial vessels protected
  - The facial vessels were identified and protected during dissection.
. Facial vessels ligated
  - Select branches of the facial vessels were ligated as indicated to facilitate exposure.

LIST: MARGINAL_MANDIBULAR_STATEMENT
. Marginal mandibular not seen (default)
  - Care was taken throughout the dissection to protect the marginal mandibular branch of the facial nerve, which was not encountered within the operative field.
. Marginal mandibular identified
  - The marginal mandibular branch of the facial nerve was identified and carefully protected throughout the dissection.
. Marginal mandibular stimulated
  - The marginal mandibular branch of the facial nerve was identified with nerve stimulation and carefully protected throughout the dissection.
</EXAMPLE RESPONSE>

<EXAMPLE PROMPT>
findings section for mandible fractures
</EXAMPLE PROMPT>

<EXAMPLE RESPONSE>
SNIPPET: op_findings_mandible_fracture
PREFIX: mandfx_find
TITLE: Findings - mandible fracture
TEXT:
- Mandibular fractures involving the ***(fracture sites) consistent with preoperative imaging.
- Fractures noted to be FRACTURE_DISPLACEMENT.
- Malocclusion present pre-reduction.
- Occlusion restored with stable bilateral posterior contacts at conclusion.
ARCH_BARS_LINE

LIST: FRACTURE_DISPLACEMENT
- displaced (default)
- minimally displaced
- comminuted
- telescoped

OPTIONAL: ARCH_BARS_LINE (default on)
  - Maxillary and mandibular arch bars in place.
</EXAMPLE RESPONSE>

<EXAMPLE PROMPT>
Exam section for an implant consult, with the shared case facts
</EXAMPLE PROMPT>

<EXAMPLE RESPONSE>
SNIPPET: common_implant_case
PREFIX: imp
TITLE: Case
TEXT:

BLANK: PATIENT_AGE (label: Age; optional)
TEETH: EDENTULOUS_SITES (label: Edentulous)
TEETH: PRESENT_TEETH (label: Tooth present; same chart as: EDENTULOUS_SITES)
BLANK: REFERRING_PROVIDER (label: Referring provider; optional)

=====

SNIPPET: clinic_exam_implant
PREFIX: imp_exam
TITLE: Exam - implant consult
USES: common_implant_case
TEXT:
?- Oral hygiene: ORAL_HYGIENE
?- Periodontal status: PERIO_STATUS
?- Adjacent teeth: ADJACENT_TEETH
?- Occlusion: OCCLUSION_STATUS
EXTRAORAL_EXAM
EXAM_SPANS
EXAM_TEETH

LIST: ORAL_HYGIENE
- good (default)
- fair
- poor
- ***

LIST: PERIO_STATUS
. Healthy
  - periodontium healthy, without generalized inflammation or mobility
. Gingivitis
  - generalized gingival inflammation without evidence of attachment loss
. Periodontitis
  - periodontitis, ***(periodontal findings; optional)
. ***
  - ***

LIST: ADJACENT_TEETH
. Sound
  - sound, without caries, mobility, or periapical pathology
. Restored, stable
  - restored, clinically stable
. ***
  - ***

LIST: OCCLUSION_STATUS
. Stable (default)
  - stable and reproducible
. Bite collapse
  - posterior bite collapse
. ***
  - ***

OPTIONAL: EXTRAORAL_EXAM (default off; label: + Extraoral/TMJ exam)
  ?- Extraoral/TMJ: EXTRAORAL_FINDINGS

PARTS(semicolon): EXTRAORAL_FINDINGS
  - face FACE_SYMMETRY
  - TMJs TMJ_STATUS
  - maximal interincisal opening MOUTH_OPENING

LIST: FACE_SYMMETRY
- symmetric (default)
- asymmetric
- ***

LIST: TMJ_STATUS
- non-tender without clicking or crepitus (default)
- with clicking
- with crepitus
- ***

LIST: MOUTH_OPENING
. Adequate (default)
  - adequate for guided surgery (>40 mm)
. Limited
  - limited, ***(MIO mm) mm
. ***
  - ***

REPEAT(newline): EXAM_SPANS (label: Edentulous span; each span of EDENTULOUS_SITES)
  ?- THIS_SITE.Site THIS_SITE: SPAN_FINDINGS

PARTS(semicolon): SPAN_FINDINGS (in EXAM_SPANS)
  - RIDGE_FORM
  - KERATINIZED_TISSUE keratinized tissue

LIST: RIDGE_FORM (in EXAM_SPANS)
. Adequate (default)
  - ridge form adequate
. Width deficient
  - horizontal ridge deficiency
. Height deficient
  - vertical ridge deficiency
. ***
  - ***

LIST: KERATINIZED_TISSUE (in EXAM_SPANS)
- adequate (default)
- limited
- no
- ***

REPEAT(newline): EXAM_TEETH (label: Tooth present; each tooth of PRESENT_TEETH)
  ?- Tooth THIS_SITE: TOOTH_CONDITIONS

MULTILIST(and): TOOTH_CONDITIONS (in EXAM_TEETH)
- heavily restored
- fractured
- carious
- mobile
- ***

PRESET: Healthy perio, sound adjacent teeth
  PERIO_STATUS = Healthy
  ADJACENT_TEETH = Sound

PRESET: Width deficient (in EXAM_SPANS)
  RIDGE_FORM = Width deficient
  KERATINIZED_TISSUE = limited
</EXAMPLE RESPONSE>

<EXAMPLE PROMPT>
Risk discussion for the implant consult, specific to the next procedure
</EXAMPLE PROMPT>

<EXAMPLE RESPONSE>
SNIPPET: clinic_risks_implant
PREFIX: imp_risk
TITLE: Risks - dental implants
USES: common_implant_case, clinic_assessment_plan_implant (optional), clinic_history_implant (optional)
TEXT:
Risks, benefits, and alternatives were discussed. Risks reviewed:
- General surgical: bleeding, infection, pain, swelling, bruising, delayed healing, and the possible need for additional procedures.
?- Specific to the planned procedure: SPECIFIC_RISKS.
The patient understands that success cannot be guaranteed. All questions were answered, and the patient verbalized understanding.

PARTS(semicolon): SPECIFIC_RISKS
  - fracture of the alveolar bone or retained root fragments (when NEXT_PROCEDURE has Extraction, Immediate implant)
  - partial graft resorption or loss requiring regrafting (when NEXT_PROCEDURE has Socket preservation)
  - failure of osseointegration requiring implant removal and replacement (when NEXT_PROCEDURE has Implant placement, Immediate implant)
  - membrane exposure, wound dehiscence, or incomplete regeneration (when NEXT_PROCEDURE has GBR)
  - altered sensation of the lip, chin, tongue, or gingiva from inferior alveolar, mental, or lingual nerve injury (when (NEXT_PROCEDURE is set and (EDENTULOUS_SITES in mandibular posterior or PRESENT_TEETH in mandibular posterior)) or ADDED_RISKS has IAN / lingual nerve)
  - higher failure and complication rates with tobacco use (when TOBACCO_USE is Current, Vaping or ADDED_RISKS has Tobacco)

MULTILIST(and): ADDED_RISKS
- IAN / lingual nerve
- Tobacco
</EXAMPLE RESPONSE>

</EXAMPLES>
