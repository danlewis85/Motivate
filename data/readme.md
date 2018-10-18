These Python notebooks relate to the specific parameterisation of bits of the ABM

1) Daily Rainfall.ipynb

Uses rainfall data for the South-East of England in conjunction with Barclay's Bike Cycle Hire data for the same period to investigate how usage differs with rainfall, and selects a daily rainfall total that is associated with the lowest levels of activity in the cycle hire data.

Then specifies a Markov chain to generate similar patterns of rainfall, both on average across the year, and by month.

2) Travel to Work.ipynb and Travel to Work By Ward.ipynb

Uses origin-destination commuting data for LSOAs and Workplace zones to look at the commuting trip distributions for Waltham Forest and for Waltham Forest Wards. Then fits a Gaussian Mixture Model to generate similar data. Ward data are first clustered using a histogram similarity measure to reduce the number of Models from 20 (the number of wards) to a more manageable 3, revealing a broadly north-south trend in commuting flows.

3) Spatial Microsimulation

The creation of synthetic data with which to create 'realistic' agents uses a spatial microsimulation approach based on Census 2011 data for constraints, and 'Understanding Society' survey data for individuals.

3.1) Constraints Data.ipynb

Creates constraints by Waltham Forest Ward based on the crosstabulation of age, sex and ethnicity, employment, and mode of travel to work, from Nomis data.

3.2) UKHLS Data.ipynb

Creates an individual dataset for spatial microsimulation from Wave 6 of 'Understanding Society' which is the most recent wave that contains commuting data. Cleans and foramts data, deriving variables for sex, age, ethnicity, employment status, commute mode, car access and bicycle access.

3.3) WF_Spatial_Microsimulation.ipynb

Takes in the constraints and individual data, and outputs a dataset of individuals who are constrained to appear similar to the Waltham Forest population.

3.4) WF Travel Distance.ipynb

Takes in the synthetic dataset created in 3.3 and generates travel distances for commuters based on Census flow data by commute mode. Having done this commuters are split into commute classes: local, city, beyond. Note that currently the demographic characteristics of agents have no effect on their commute distance, just their mode of travel.

4) Neighbourhood Environment Scores

Generates scores for the 20 wards in Waltham forest based on smple assumptions about their supportiveness for different modes of travel.

4.1) tbd

5) tbd

Takes in the full synthetic dataset output in 3.4 and creates a yaml file that can be read into the Motivate ABM.

